//! The audio thread's half: host parameter changes become writes.
//!
//! Nothing here locks, allocates or makes a syscall: [`ParamBank::poll`]
//! reads atomics and [`Sender::write`] pushes to a wait-free ring.

use integral_device_service::Sender;

use super::instance::Instance;

/// What `process()` owns to send host changes to the device.
pub struct Outbound {
    sender: Sender,
}

impl Outbound {
    /// An outbound path over `sender`, which the connection made off the
    /// audio thread ([`Connection::sender`](super::link::Connection::sender)).
    pub fn new(sender: Sender) -> Self {
        Self { sender }
    }

    /// Send the host's changes since the last call: one typed write for each
    /// parameter that changed, scoped by the instance's Binding. Nothing is
    /// sent for a change that is an echo of a value the instance applied
    /// from the device, for a parameter that does nothing now (an inactive
    /// effect type, a Binding the plugin does not use it for), while the
    /// instance is unbound, or while it is not armed
    /// ([`Instance::arm`](Instance::arm)).
    ///
    /// Real-time safe.
    pub fn poll(&mut self, instance: &Instance) {
        let sender = &mut self.sender;
        instance.bank.poll(
            instance.binding.get(),
            instance.is_armed(),
            |handle, binding, value| sender.write(handle, binding, value),
        );
    }
}
