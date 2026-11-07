//! Liveness of a MIDI port pair, by Identity Request.
//!
//! A MIDI port that exists says nothing about the device behind it: over
//! RTP-MIDI through a hub, a power-cycled INTEGRA-7 leaves the port in place.
//! So the link sends an Identity Request when it opens the pair and then
//! periodically, reports up on the first Identity Reply (which also tells the
//! device ID), and reports down after too many probes go unanswered.
//!
//! [`Liveness`] is sans-IO: time comes from the caller.

use integral_core::sysex;

/// Probe timing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Timing {
    /// How long the device may be silent before a probe (ms), and the time
    /// between probes after that.
    pub interval_ms: f64,
    /// The link is down once this many probes in a row went unanswered.
    pub max_missed: u32,
}

impl Default for Timing {
    /// A second of silence, then a probe a second, down after three
    /// unanswered: a vanished device is noticed within about four seconds.
    fn default() -> Self {
        Self {
            interval_ms: 1000.0,
            max_missed: 3,
        }
    }
}

/// What the link should do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Send an Identity Request.
    Probe,
    /// An INTEGRA-7 answered: the link is up, and its device ID is this.
    Up(u8),
    /// The device stopped answering: the link is down.
    Down,
}

/// Tracks whether the device behind a port pair answers.
///
/// While down it probes every interval. While up it probes only after the
/// device has been silent for an interval: any message from the device,
/// Active Sensing included, counts as life and postpones the next probe. So
/// a quiet link sees a probe a second at most, and a busy one none. The cost
/// is that other traffic on a shared RTP-MIDI session can mask a power cycle.
#[derive(Debug)]
pub struct Liveness {
    timing: Timing,
    up: bool,
    next_probe: f64,
    /// Probes sent since the device was last heard.
    missed: u32,
}

impl Liveness {
    /// Start probing at `now_ms`; the first probe is due at once.
    pub fn new(timing: Timing, now_ms: f64) -> Self {
        Self {
            timing,
            up: false,
            next_probe: now_ms,
            missed: 0,
        }
    }

    /// Whether the device is believed reachable.
    pub fn is_up(&self) -> bool {
        self.up
    }

    /// When [`tick`](Self::tick) next has something to do.
    pub fn next_deadline(&self) -> f64 {
        self.next_probe
    }

    /// Advance to `now_ms`: probe if one is due, and give up on a device that
    /// has left too many probes unanswered.
    pub fn tick(&mut self, now_ms: f64) -> Vec<Action> {
        let mut actions = Vec::new();
        if now_ms < self.next_probe {
            return actions;
        }
        if self.up && self.missed >= self.timing.max_missed {
            self.up = false;
            actions.push(Action::Down);
        }
        self.missed += 1;
        self.next_probe = now_ms + self.timing.interval_ms;
        actions.push(Action::Probe);
        actions
    }

    /// Account for one message from the device at `now_ms`. While up, any
    /// message proves the device is there, so it postpones the next probe.
    pub fn on_message(&mut self, message: &[u8], now_ms: f64) -> Option<Action> {
        if self.up {
            self.missed = 0;
            self.next_probe = now_ms + self.timing.interval_ms;
            return None;
        }
        let identity = sysex::parse_identity_reply(message).ok()?;
        if !identity.is_integra7() {
            return None;
        }
        self.up = true;
        self.missed = 0;
        self.next_probe = now_ms + self.timing.interval_ms;
        Some(Action::Up(identity.device_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const REPLY: [u8; 15] = [
        0xF0, 0x7E, 0x12, 0x06, 0x02, 0x41, 0x64, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF7,
    ];

    #[test]
    fn probes_until_an_integra7_answers() {
        let mut live = Liveness::new(Timing::default(), 0.0);
        assert_eq!(live.tick(0.0), vec![Action::Probe]);
        assert_eq!(live.tick(500.0), vec![]);
        assert_eq!(live.on_message(&[0x90, 60, 1], 600.0), None);
        assert_eq!(live.on_message(&sysex::identity_request(), 700.0), None);
        assert!(!live.is_up());
        assert_eq!(live.tick(1000.0), vec![Action::Probe]);
        assert_eq!(live.on_message(&REPLY, 1010.0), Some(Action::Up(0x12)));
        assert!(live.is_up());
    }

    #[test]
    fn another_device_does_not_bring_the_link_up() {
        let mut other = REPLY;
        other[5] = 0x43; // not Roland
        let mut live = Liveness::new(Timing::default(), 0.0);
        assert_eq!(live.on_message(&other, 0.0), None);
    }

    #[test]
    fn goes_down_after_max_missed_unanswered_probes() {
        let timing = Timing {
            interval_ms: 1000.0,
            max_missed: 2,
        };
        let mut live = Liveness::new(timing, 0.0);
        live.tick(0.0);
        live.on_message(&REPLY, 0.0);
        // A second of silence, then two probes go unanswered.
        assert_eq!(live.tick(1000.0), vec![Action::Probe]);
        assert_eq!(live.tick(2000.0), vec![Action::Probe]);
        assert_eq!(live.tick(3000.0), vec![Action::Down, Action::Probe]);
        assert!(!live.is_up());
        // And comes back up on the next reply.
        assert_eq!(live.on_message(&REPLY, 3010.0), Some(Action::Up(0x12)));
    }

    #[test]
    fn a_chatty_device_is_never_probed() {
        let mut live = Liveness::new(Timing::default(), 0.0);
        live.tick(0.0);
        live.on_message(&REPLY, 0.0);
        // Active Sensing every 250 ms for a minute: no probe, still up.
        let mut now = 0.0;
        while now < 60_000.0 {
            now += 250.0;
            assert_eq!(live.tick(now), vec![], "probe at {now}");
            live.on_message(&[0xFE], now);
        }
        assert!(live.is_up());
    }

    #[test]
    fn silence_after_chatter_is_probed_then_dropped() {
        let timing = Timing {
            interval_ms: 1000.0,
            max_missed: 1,
        };
        let mut live = Liveness::new(timing, 0.0);
        live.tick(0.0);
        live.on_message(&REPLY, 0.0);
        live.on_message(&[0xFE], 5_000.0);
        assert_eq!(live.tick(5_999.0), vec![]);
        assert_eq!(live.tick(6_000.0), vec![Action::Probe]);
        assert_eq!(live.tick(7_000.0), vec![Action::Down, Action::Probe]);
    }
}
