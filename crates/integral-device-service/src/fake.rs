//! A fake link for tests: records what the service sends.
//! Behind the `test-util` feature.

use std::sync::{Arc, Mutex};

use crate::link::{DeviceLink, LinkError, LinkSink};

#[derive(Default)]
struct Shared {
    sent: Vec<Vec<u8>>,
    sink: Option<LinkSink>,
    gap: std::time::Duration,
}

/// A [`DeviceLink`] that records every message sent. Clones share the
/// recording, so a test keeps one while the service owns the other.
#[derive(Clone, Default)]
pub struct FakeLink(Arc<Mutex<Shared>>);

impl FakeLink {
    /// A link that has sent nothing.
    pub fn new() -> Self {
        Self::default()
    }

    /// Every message sent so far, in order.
    pub fn sent(&self) -> Vec<Vec<u8>> {
        self.0.lock().expect("fake link lock").sent.clone()
    }

    /// Make the link ask the service to hold SysEx back for `gap`, as a
    /// link does after its own probe.
    pub fn hold_sysex(&self, gap: std::time::Duration) {
        self.0.lock().expect("fake link lock").gap = gap;
    }

    /// Forget what was sent.
    pub fn clear(&self) {
        self.0.lock().expect("fake link lock").sent.clear();
    }

    /// Report the link up, once the service has started it.
    pub fn bring_up(&self, device_id: Option<u8>) {
        self.sink().up(device_id);
    }

    /// Report the link down.
    pub fn bring_down(&self) {
        self.sink().down();
    }

    /// Report a message from the device.
    pub fn device_says(&self, message: &[u8]) {
        self.sink().midi(message);
    }

    fn sink(&self) -> LinkSink {
        self.0
            .lock()
            .expect("fake link lock")
            .sink
            .clone()
            .expect("the service has not started the link")
    }
}

impl DeviceLink for FakeLink {
    fn start(&mut self, events: LinkSink) -> Result<(), LinkError> {
        self.0.lock().expect("fake link lock").sink = Some(events);
        Ok(())
    }

    fn sysex_gap(&self) -> std::time::Duration {
        self.0.lock().expect("fake link lock").gap
    }

    fn send(&mut self, message: &[u8]) -> Result<(), LinkError> {
        self.0
            .lock()
            .expect("fake link lock")
            .sent
            .push(message.to_vec());
        Ok(())
    }
}
