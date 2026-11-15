//! `init`: reset an area of the Temporary memory to INIT (Studio Set, Sound
//! Ctrl, Part, Tone or Partial), wait for the device's "finished", and say
//! what was reset.
//!
//! The request and the wait are [`integral_core::initialize`]'s
//! [`Initialization`]; this module turns the arguments into its
//! [`InitTarget`], reads the Part's tone type when a Partial Init needs it,
//! and does the I/O.

use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use clap::{Args, ValueEnum};
use integral_core::device_spec::INTEGRA7;
use integral_core::initialize::{InitAction, InitError, InitTarget, Initialization};
use integral_core::params::{self, ToneType};
use integral_core::sysex;

use super::tone_waves::tone_type_name;
use crate::device::IntegraDevice;
use crate::midi::{self, Endpoint};

/// Exit code when the Init was sent but "finished" never came: the area's
/// content is unknown.
pub const EXIT_INIT_UNCONFIRMED: u8 = 2;

/// The area `init` resets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum InitKind {
    /// The Temporary Studio Set, every Part's tone included.
    StudioSet,
    /// The Sound Ctrl offsets of every Part.
    SoundCtrl,
    /// One Part's settings (`--part`); its tone reloads.
    Part,
    /// One Part's Temporary tone (`--part`).
    Tone,
    /// One partial (`--partial`) of a tone, or one key (`--key`) of a drum
    /// kit, on a Part (`--part`).
    Partial,
}

/// The arguments of `integral init`.
#[derive(Debug, Args)]
pub struct InitArgs {
    /// What to reset.
    #[arg(value_enum, value_name = "TARGET")]
    pub kind: InitKind,
    /// The Part (1-16) for `part`, `tone` and `partial`.
    #[arg(long, value_name = "N",
          value_parser = clap::value_parser!(u8).range(1..=i64::from(INTEGRA7.part_count)))]
    pub part: Option<u8>,
    /// The partial (from 1) of an SN Synth or PCM Synth tone, for `partial`.
    #[arg(long, value_name = "N", conflicts_with = "key",
          value_parser = clap::value_parser!(u8).range(1..))]
    pub partial: Option<u8>,
    /// The MIDI key number of a drum kit's key, for `partial`.
    #[arg(long, value_name = "N", value_parser = clap::value_parser!(u8).range(0..=127))]
    pub key: Option<u8>,
}

/// Which partial a Partial Init names, as typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PartialPick {
    /// `--partial`, 0-based.
    Index(u8),
    /// `--key`, a MIDI key number.
    Key(u8),
}

/// What the arguments ask for. A Partial Init needs the Part's tone type to
/// become an [`InitTarget`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Request {
    /// Ready to send.
    Target(InitTarget),
    /// A partial or key of the tone on a Part (0-based).
    Partial { part: u8, pick: PartialPick },
}

impl InitArgs {
    /// Check the options against the target, before opening a port.
    fn request(&self) -> Result<Request> {
        let possible = self.kind.to_possible_value();
        let name = possible.as_ref().map_or("", |v| v.get_name());
        if self.kind != InitKind::Partial && (self.partial.is_some() || self.key.is_some()) {
            bail!("--partial and --key only apply to init partial");
        }
        let whole_set = |target| {
            if self.part.is_some() {
                bail!("init {name} resets every Part: it takes no --part");
            }
            Ok(Request::Target(target))
        };
        // The 0-based Part a Part-wide target names.
        let part = || {
            self.part
                .map(|p| p - 1)
                .with_context(|| format!("init {name} needs --part N"))
        };
        match self.kind {
            InitKind::StudioSet => whole_set(InitTarget::StudioSet),
            InitKind::SoundCtrl => whole_set(InitTarget::SoundCtrl),
            InitKind::Part => Ok(Request::Target(InitTarget::Part(part()?))),
            InitKind::Tone => Ok(Request::Target(InitTarget::Tone(part()?))),
            InitKind::Partial => {
                let part = part()?;
                let pick = match (self.partial, self.key) {
                    (Some(n), None) => PartialPick::Index(n - 1),
                    (None, Some(key)) => PartialPick::Key(key),
                    _ => bail!(
                        "init partial needs --partial N (a tone's partial) or --key N (a kit's key)"
                    ),
                };
                Ok(Request::Partial { part, pick })
            }
        }
    }
}

/// The Partial Init of `pick` on `part`, which plays a `tone` tone.
fn partial_target(part: u8, pick: PartialPick, tone: ToneType) -> Result<InitTarget> {
    let name = tone_type_name(tone);
    let n = match (pick, tone.is_kit()) {
        (PartialPick::Index(i), false) => i,
        (PartialPick::Key(k), true) => k,
        (PartialPick::Index(_), true) => {
            bail!(
                "Part {} plays a kit ({name}): name its key with --key",
                part + 1
            )
        }
        (PartialPick::Key(_), false) => bail!(
            "Part {} plays a tone ({name}), not a kit: name its partial with --partial",
            part + 1
        ),
    };
    InitTarget::partial(part, tone, n).map_err(|e| match (e, pick) {
        (InitError::InvalidPartial(_), PartialPick::Index(i)) => {
            anyhow::anyhow!("the tone ({name}) has no partial {}", i + 1)
        }
        (InitError::InvalidPartial(_), PartialPick::Key(k)) => {
            anyhow::anyhow!("the kit ({name}) has no key {k}")
        }
        (InitError::NoPartials, _) => anyhow::anyhow!("the tone ({name}) has no partials"),
        (e, _) => e.into(),
    })
}

/// What was reset, in one sentence. `tone` names the Part's tone type when
/// it was read.
fn describe(target: InitTarget, tone: Option<ToneType>) -> String {
    let kind = tone.map_or("", tone_type_name);
    match target {
        InitTarget::StudioSet => "Reset the Studio Set to INIT, every Part's tone included.".into(),
        InitTarget::SoundCtrl => format!(
            "Reset the Sound Ctrl offsets of all {} Parts to INIT.",
            INTEGRA7.part_count
        ),
        InitTarget::Part(p) => format!("Reset Part {} to INIT; its tone reloaded.", p + 1),
        InitTarget::Tone(p) => format!("Reset Part {}'s tone to INIT.", p + 1),
        InitTarget::Partial { part, partial } => match tone {
            Some(t) if t.is_kit() => format!(
                "Reset key {} of Part {}'s kit ({kind}) to INIT.",
                partial.value(),
                part + 1
            ),
            _ => format!(
                "Reset partial {} of Part {}'s tone ({kind}) to INIT.",
                partial.value() + 1,
                part + 1
            ),
        },
    }
}

/// Run `init` to completion: send what it asks for through `send`, feed it
/// the DT1s from `rx`, and wait `wait(timeout_ms)` for "finished". `Err` is
/// a transport failure; the inner result is the Init's.
fn drive(
    init: &mut Initialization,
    mut send: impl FnMut(&[u8]) -> Result<()>,
    rx: &Receiver<Vec<u8>>,
    wait: impl Fn(f64) -> Duration,
) -> Result<Result<(), InitError>> {
    loop {
        match init.next_action() {
            InitAction::Send(message) => send(&message).context("failed to send the Init")?,
            InitAction::Await { timeout_ms } => {
                let deadline = Instant::now() + wait(timeout_ms);
                loop {
                    match rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                        Ok(raw) => {
                            if let Ok(dt1) = sysex::parse_dt1(&raw)
                                && init.on_reply(&dt1.address, &dt1.data)
                            {
                                break;
                            }
                        }
                        Err(RecvTimeoutError::Timeout) => {
                            init.on_timeout();
                            break;
                        }
                        Err(RecvTimeoutError::Disconnected) => bail!("MIDI input disconnected"),
                    }
                }
            }
            InitAction::Done(result) => return Ok(result),
        }
    }
}

/// `integral init`: check the arguments, reset the area, wait for
/// "finished" and print what was reset.
///
/// Returns the exit code: 0 when the device confirmed the Init, or
/// [`EXIT_INIT_UNCONFIRMED`] when it was sent but not confirmed. `Err`
/// (exit 1) means nothing was sent.
pub fn run(
    endpoint: Endpoint<'_>,
    device_id: Option<u8>,
    timeout: Duration,
    args: &InitArgs,
) -> Result<u8> {
    let request = args.request()?;
    let mut dev = IntegraDevice::connect_detecting(endpoint, device_id, timeout)?.quiet();
    let (target, tone) = match request {
        Request::Target(target) => (target, None),
        Request::Partial { part, pick } => {
            let tone = read_tone_type(&mut dev, part)?;
            (partial_target(part, pick, tone)?, Some(tone))
        }
    };
    let mut init = Initialization::new(dev.device_id(), target)?;
    // Through the service our Init may queue behind other clients' SysEx.
    let service = matches!(endpoint, Endpoint::Service(_));
    let wait = |ms: f64| midi::timeout(None, ms / 1000.0, service);
    let (out, rx) = dev.split();
    match drive(&mut init, |m| out.send(m), rx, wait)? {
        Ok(()) => {
            println!("{}", describe(target, tone));
            println!("Only Temporary memory changed: stored User slots are untouched.");
            Ok(0)
        }
        Err(e) => {
            eprintln!("Error: {e}: the area's content is unknown; read it back.");
            Ok(EXIT_INIT_UNCONFIRMED)
        }
    }
}

/// The tone type on Part `part` (0-based), from its Bank Select MSB.
fn read_tone_type(dev: &mut IntegraDevice, part: u8) -> Result<ToneType> {
    let bank_addr = params::part_address(part, params::part::TONE_BANK_MSB);
    let msb = dev.request_data(&bank_addr, &params::SINGLE_BYTE_SIZE)?[0];
    params::tone_type_from_bank_msb(msb)
        .with_context(|| format!("unknown tone type on Part {} (bank MSB {msb})", part + 1))
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use integral_core::initialize::{INIT_FINISHED_ADDR, INIT_STARTED_ADDR, build_init_request};
    use integral_core::sysex::DEFAULT_DEVICE_ID;

    use super::*;

    fn args(kind: InitKind, part: Option<u8>, partial: Option<u8>, key: Option<u8>) -> InitArgs {
        InitArgs {
            kind,
            part,
            partial,
            key,
        }
    }

    fn err(args: &InitArgs) -> String {
        args.request().unwrap_err().to_string()
    }

    #[test]
    fn whole_set_targets_take_no_part() {
        use InitKind::*;
        assert_eq!(
            args(StudioSet, None, None, None).request().unwrap(),
            Request::Target(InitTarget::StudioSet)
        );
        assert_eq!(
            args(SoundCtrl, None, None, None).request().unwrap(),
            Request::Target(InitTarget::SoundCtrl)
        );
        for kind in [StudioSet, SoundCtrl] {
            assert!(err(&args(kind, Some(3), None, None)).contains("takes no --part"));
        }
    }

    #[test]
    fn part_targets_need_a_part() {
        use InitKind::*;
        for kind in [Part, Tone, Partial] {
            assert!(err(&args(kind, None, None, None)).contains("needs --part N"));
        }
        assert_eq!(
            args(Part, Some(16), None, None).request().unwrap(),
            Request::Target(InitTarget::Part(15))
        );
        assert_eq!(
            args(Tone, Some(3), None, None).request().unwrap(),
            Request::Target(InitTarget::Tone(2))
        );
    }

    #[test]
    fn only_partial_takes_partial_or_key() {
        use InitKind::*;
        assert!(err(&args(Tone, Some(1), Some(1), None)).contains("only apply to init partial"));
        assert!(err(&args(Part, Some(1), None, Some(36))).contains("only apply to init partial"));
        assert!(err(&args(Partial, Some(1), None, None)).contains("--partial N"));
        assert_eq!(
            args(Partial, Some(2), Some(3), None).request().unwrap(),
            Request::Partial {
                part: 1,
                pick: PartialPick::Index(2)
            }
        );
        assert_eq!(
            args(Partial, Some(2), None, Some(36)).request().unwrap(),
            Request::Partial {
                part: 1,
                pick: PartialPick::Key(36)
            }
        );
    }

    #[test]
    fn key_on_a_non_kit_is_refused() {
        for tone in [ToneType::PcmSynth, ToneType::SnSynth, ToneType::SnAcoustic] {
            let e = partial_target(2, PartialPick::Key(36), tone)
                .unwrap_err()
                .to_string();
            assert!(e.contains("not a kit") && e.contains("--partial"), "{e}");
        }
    }

    #[test]
    fn partial_on_a_kit_is_refused() {
        for kit in [ToneType::PcmDrum, ToneType::SnDrum] {
            let e = partial_target(2, PartialPick::Index(0), kit)
                .unwrap_err()
                .to_string();
            assert!(e.contains("--key"), "{e}");
        }
    }

    #[test]
    fn partial_and_key_are_checked_against_the_tone() {
        assert_eq!(
            partial_target(2, PartialPick::Index(1), ToneType::PcmSynth).unwrap(),
            InitTarget::partial(2, ToneType::PcmSynth, 1).unwrap()
        );
        assert_eq!(
            partial_target(15, PartialPick::Key(36), ToneType::PcmDrum).unwrap(),
            InitTarget::partial(15, ToneType::PcmDrum, 36).unwrap()
        );
        let past = INTEGRA7.pcm_synth_partial_count;
        let e = partial_target(0, PartialPick::Index(past), ToneType::PcmSynth)
            .unwrap_err()
            .to_string();
        assert_eq!(
            e,
            format!("the tone (pcm-synth) has no partial {}", past + 1)
        );
        let e = partial_target(0, PartialPick::Key(0), ToneType::SnDrum)
            .unwrap_err()
            .to_string();
        assert_eq!(e, "the kit (sn-drum) has no key 0");
        let e = partial_target(0, PartialPick::Index(0), ToneType::SnAcoustic)
            .unwrap_err()
            .to_string();
        assert!(e.contains("no partials"), "{e}");
    }

    #[test]
    fn describes_what_was_reset() {
        assert_eq!(
            describe(InitTarget::SoundCtrl, None),
            format!(
                "Reset the Sound Ctrl offsets of all {} Parts to INIT.",
                INTEGRA7.part_count
            )
        );
        assert_eq!(
            describe(InitTarget::Tone(2), None),
            "Reset Part 3's tone to INIT."
        );
        assert_eq!(
            describe(
                InitTarget::partial(15, ToneType::PcmDrum, 36).unwrap(),
                Some(ToneType::PcmDrum)
            ),
            "Reset key 36 of Part 16's kit (pcm-drum) to INIT."
        );
        assert_eq!(
            describe(
                InitTarget::partial(0, ToneType::SnSynth, 2).unwrap(),
                Some(ToneType::SnSynth)
            ),
            "Reset partial 3 of Part 1's tone (sn-synth) to INIT."
        );
    }

    const SHORT: fn(f64) -> Duration = |_| Duration::from_millis(20);

    /// The device's two replies, with an unrelated DT1 between them.
    #[test]
    fn drive_sends_then_waits_for_finished() {
        let target = InitTarget::Part(15);
        let mut init = Initialization::new(DEFAULT_DEVICE_ID, target).unwrap();
        let (tx, rx) = mpsc::channel();
        let mut sent = Vec::new();
        let result = drive(
            &mut init,
            |m| {
                sent.push(m.to_vec());
                tx.send(sysex::build_dt1(
                    DEFAULT_DEVICE_ID,
                    &INIT_STARTED_ADDR,
                    &[0; 4],
                ))?;
                tx.send(sysex::build_dt1(
                    DEFAULT_DEVICE_ID,
                    &INIT_FINISHED_ADDR,
                    &[0, 0, 0, 1],
                ))?;
                tx.send(sysex::build_dt1(
                    DEFAULT_DEVICE_ID,
                    &INIT_FINISHED_ADDR,
                    &[0; 4],
                ))?;
                Ok(())
            },
            &rx,
            SHORT,
        )
        .unwrap();
        assert_eq!(result, Ok(()));
        assert_eq!(sent, [build_init_request(DEFAULT_DEVICE_ID, target)]);
    }

    #[test]
    fn drive_times_out_without_finished() {
        let mut init = Initialization::new(DEFAULT_DEVICE_ID, InitTarget::Tone(0)).unwrap();
        let (tx, rx) = mpsc::channel::<Vec<u8>>();
        let result = drive(
            &mut init,
            |_| {
                Ok(tx.send(sysex::build_dt1(
                    DEFAULT_DEVICE_ID,
                    &INIT_STARTED_ADDR,
                    &[0; 4],
                ))?)
            },
            &rx,
            SHORT,
        )
        .unwrap();
        assert_eq!(result, Err(InitError::Timeout));
    }

    #[test]
    fn drive_fails_when_the_input_drops() {
        let mut init = Initialization::new(DEFAULT_DEVICE_ID, InitTarget::Tone(0)).unwrap();
        let (tx, rx) = mpsc::channel::<Vec<u8>>();
        drop(tx);
        assert!(drive(&mut init, |_| Ok(()), &rx, SHORT).is_err());
    }
}
