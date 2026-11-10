//! Running a [`Service`] on its own thread.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::Instant;

use thiserror::Error;

use crate::link::{LinkError, LinkSink};
use crate::note::{NotePort, NoteSink, PortError};
use crate::protocol::ClientId;
use crate::service::{Input, Service};

/// Why the service did not start.
#[derive(Debug, Error)]
pub enum StartError {
    /// The link could not start.
    #[error("link: {0}")]
    Link(#[from] LinkError),
    /// The note port could not open.
    #[error(transparent)]
    NotePort(#[from] PortError),
    /// The thread could not be spawned.
    #[error("thread: {0}")]
    Thread(#[from] std::io::Error),
}

/// A way to reach a running [`Service`] from another thread. Cheap to
/// clone. It can run work on the service thread but cannot forge link
/// events, notes or a stop.
#[derive(Clone)]
pub struct Handle {
    tx: Sender<Input>,
    /// The last client ID given out, shared by every clone, so the socket
    /// and WebSocket servers never give two clients one ID.
    last_client: Arc<AtomicU64>,
}

impl Handle {
    /// Run `work` against the service on its thread. Returns `false` if the
    /// service has stopped.
    pub fn run(&self, work: impl FnOnce(&mut Service) + Send + 'static) -> bool {
        self.tx.send(Input::Run(Box::new(work))).is_ok()
    }

    /// Feed a client's input (connect, request, disconnect) to the service.
    /// Crate-private: the socket layer is the one source of these, so a
    /// handle still cannot forge link events, notes or a stop.
    pub(crate) fn client_input(&self, input: Input) -> bool {
        self.tx.send(input).is_ok()
    }

    /// Feed one channel voice message from a WebSocket client to the
    /// service, as if it arrived on the note port: it goes out between SysEx
    /// messages, ahead of the SysEx queue. Crate-private, like
    /// [`client_input`](Self::client_input): only the WebSocket endpoint,
    /// which admits allowed origins alone and checks each frame, may play
    /// notes, so a handle still cannot forge them.
    pub(crate) fn client_note(&self, message: Vec<u8>) -> bool {
        self.tx.send(Input::Note(message)).is_ok()
    }

    /// A client ID no other connection has had. `ClientId::DEVICE` (0) is
    /// never given out.
    pub(crate) fn new_client_id(&self) -> ClientId {
        ClientId(self.last_client.fetch_add(1, Ordering::SeqCst) + 1)
    }
}

/// A [`Service`] running on its thread.
pub struct Running {
    handle: Handle,
    thread: JoinHandle<()>,
}

impl Running {
    /// A handle to feed the service.
    pub fn handle(&self) -> Handle {
        self.handle.clone()
    }

    /// Ask the service to stop and wait for it. Closes the note port; the
    /// link stops with the service.
    pub fn stop(self) {
        let _ = self.handle.tx.send(Input::Stop);
        let _ = self.thread.join();
    }

    /// Wait until the service stops.
    pub fn join(self) {
        let _ = self.thread.join();
    }
}

/// Start `service` on a new thread: start its link, open the note port, then
/// pump until stopped. `notes` is optional so a service without a virtual
/// port (a test, a platform without one yet) still runs.
///
/// # Errors
///
/// [`StartError`] if the link, the note port or the thread fails to start.
pub fn spawn(
    mut service: Service,
    notes: Option<Box<dyn NotePort>>,
) -> Result<Running, StartError> {
    let (tx, rx) = mpsc::channel();
    let link_tx = tx.clone();
    service.link_mut().start(LinkSink::new(move |event| {
        let _ = link_tx.send(Input::Link(event));
    }))?;
    let mut notes = notes;
    if let Some(port) = notes.as_mut() {
        let note_tx = tx.clone();
        port.open(NoteSink::new(move |message| {
            let _ = note_tx.send(Input::Note(message.to_vec()));
        }))?;
    }
    let thread = thread::Builder::new()
        .name("device-service".into())
        .spawn(move || {
            let _notes = notes; // the port lives as long as the service
            let start = Instant::now();
            let now = || start.elapsed().as_secs_f64() * 1000.0;
            loop {
                let input = match service.pump(now()) {
                    Some(wait) => rx.recv_timeout(wait),
                    None => rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
                };
                match input {
                    Ok(input) => {
                        if !service.handle(input, now()) {
                            break;
                        }
                    }
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            }
        })?;
    Ok(Running {
        handle: Handle {
            tx,
            last_client: Arc::new(AtomicU64::new(ClientId::DEVICE.0)),
        },
        thread,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FakeLink;
    use crate::service::Config;
    use std::collections::HashSet;

    #[test]
    fn client_ids_are_unique_across_clones_and_threads() {
        let running = spawn(
            Service::new(Box::new(FakeLink::new()), Config::new(Some(0x10))),
            None,
        )
        .expect("start");
        let threads: Vec<_> = (0..4)
            .map(|_| {
                let handle = running.handle();
                thread::spawn(move || (0..100).map(|_| handle.new_client_id()).collect::<Vec<_>>())
            })
            .collect();
        let ids: Vec<_> = threads
            .into_iter()
            .flat_map(|t| t.join().expect("thread"))
            .collect();
        running.stop();

        assert_eq!(ids.iter().collect::<HashSet<_>>().len(), ids.len());
        assert!(!ids.contains(&ClientId::DEVICE));
    }
}
