//! Running a [`Service`] on its own thread.

use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::Instant;

use thiserror::Error;

use crate::link::{LinkError, LinkSink};
use crate::note::{NotePort, NoteSink, PortError};
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
pub struct Handle(Sender<Input>);

impl Handle {
    /// Run `work` against the service on its thread. Returns `false` if the
    /// service has stopped.
    pub fn run(&self, work: impl FnOnce(&mut Service) + Send + 'static) -> bool {
        self.0.send(Input::Run(Box::new(work))).is_ok()
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
        let _ = self.handle.0.send(Input::Stop);
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
        handle: Handle(tx),
        thread,
    })
}
