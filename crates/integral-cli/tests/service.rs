//! `--service` end to end: the `integral` binary against a Device service
//! running in this process over a `FakeLink`.
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use integral_core::params::{self, STUDIO_SET_NAME, part};
use integral_core::sysex;
use integral_device_service::{
    Config, FakeLink, Running, SequentialSessionIds, Service, SocketServer, ensure_private_dir,
    socket_path_in, spawn,
};

/// The device ID the service uses (and the fake device answers with).
const DEVICE_ID: u8 = 0x17;
/// A different one, given to the CLI: the service must win.
const CLI_DEVICE_ID: &str = "20";
const WAIT: Duration = Duration::from_secs(10);

/// A private socket directory of its own under the target directory,
/// removed on drop.
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("cli{}-{name}", std::process::id()));
        ensure_private_dir(&dir).expect("private temp dir");
        Self(dir)
    }

    fn socket(&self) -> PathBuf {
        socket_path_in(&self.0)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A service on a fake link, serving a socket.
struct Fixture {
    _dir: TempDir,
    socket: PathBuf,
    link: FakeLink,
    server: Option<SocketServer>,
    running: Option<Running>,
}

impl Fixture {
    fn start(name: &str) -> Self {
        let dir = TempDir::new(name);
        let link = FakeLink::new();
        let config = Config {
            device_id: Some(DEVICE_ID),
            session_ids: Box::new(SequentialSessionIds(1)),
        };
        let running = spawn(Service::new(Box::new(link.clone()), config), None).expect("start");
        link.bring_up(None);
        let socket = dir.socket();
        let server = SocketServer::bind(&socket, running.handle()).expect("bind");
        Self {
            _dir: dir,
            socket,
            link,
            server: Some(server),
            running: Some(running),
        }
    }

    /// `integral --service=<socket> <args>`, with its output piped.
    fn integral(&self, args: &[&str]) -> Command {
        integral(&self.socket, args)
    }

    /// Wait until the link was sent `message`.
    fn wait_sent(&self, message: &[u8]) {
        let deadline = Instant::now() + WAIT;
        while !self.link.sent().iter().any(|m| m == message) {
            assert!(
                Instant::now() < deadline,
                "{message:02X?} never reached the link"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// Stop the service as a clean exit would.
    fn stop(&mut self) {
        self.server.take();
        if let Some(running) = self.running.take() {
            running.stop();
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop();
    }
}

fn integral(socket: &Path, args: &[&str]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_integral"));
    command
        .arg(format!("--service={}", socket.display()))
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

/// Run to the end and return (success, stdout, stderr).
fn finish(child: Child) -> (bool, String, String) {
    let output = child.wait_with_output().expect("integral exits");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn ping_is_answered_through_the_service() {
    let fixture = Fixture::start("ping");
    let child = fixture.integral(&["ping"]).spawn().expect("run integral");

    fixture.wait_sent(&sysex::identity_request());
    // An Identity Reply: universal non-realtime, general info, reply, Roland,
    // family, number, revision.
    fixture.link.device_says(&[
        0xF0, 0x7E, DEVICE_ID, 0x06, 0x02, 0x41, 0x64, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0xF7,
    ]);

    let (ok, stdout, stderr) = finish(child);
    assert!(ok, "{stderr}");
    assert!(stdout.contains("Device ID:     17H"), "{stdout}");
}

/// A fire-and-forget send is still on its way when the CLI exits.
#[test]
fn a_write_reaches_the_link_before_the_cli_exits() {
    let fixture = Fixture::start("write");
    let child = fixture
        .integral(&[
            "write",
            "part-level",
            "--part",
            "1",
            "--value",
            "100",
            "--device-id",
            "17",
        ])
        .spawn()
        .expect("run integral");
    let (ok, _, stderr) = finish(child);
    assert!(ok, "{stderr}");

    let dt1 = sysex::build_dt1(DEVICE_ID, &params::part_address(0, part::LEVEL), &[100]);
    fixture.wait_sent(&dt1);
}

/// The service speaks to the device with its own ID, whatever the CLI says.
#[test]
fn a_read_is_answered_whatever_device_id_the_cli_gives() {
    let fixture = Fixture::start("read");
    let child = fixture
        .integral(&["raw-read", "--device-id", CLI_DEVICE_ID, "18000000", "16"])
        .spawn()
        .expect("run integral");

    let size = integral_core::address::DataSize::from_value(16).expect("size");
    fixture.wait_sent(&sysex::build_rq1(DEVICE_ID, &STUDIO_SET_NAME, &size));
    fixture.link.device_says(&sysex::build_dt1(
        DEVICE_ID,
        &STUDIO_SET_NAME,
        b"Test Set        ",
    ));

    let (ok, stdout, stderr) = finish(child);
    assert!(ok, "{stderr}");
    assert!(stdout.contains("Test Set"), "{stdout}");
}

#[test]
fn a_service_that_goes_away_mid_read_fails_the_read() {
    let mut fixture = Fixture::start("gone");
    let started = Instant::now();
    let child = fixture
        .integral(&[
            "raw-read",
            "--device-id",
            "17",
            "--timeout",
            "30",
            "18000000",
            "16",
        ])
        .spawn()
        .expect("run integral");

    let size = integral_core::address::DataSize::from_value(16).expect("size");
    fixture.wait_sent(&sysex::build_rq1(DEVICE_ID, &STUDIO_SET_NAME, &size));
    fixture.stop();

    let (ok, _, stderr) = finish(child);
    assert!(!ok, "the read succeeded without a service");
    assert!(started.elapsed() < Duration::from_secs(20), "{stderr}");
}

#[test]
fn no_service_at_the_socket_is_reported() {
    let dir = TempDir::new("none");
    let (ok, _, stderr) = finish(
        integral(&dir.socket(), &["ping"])
            .spawn()
            .expect("run integral"),
    );
    assert!(!ok);
    assert!(stderr.contains("no Device service"), "{stderr}");
}

#[test]
fn commands_that_need_ports_refuse_the_service() {
    let dir = TempDir::new("refuse");
    for args in [&["monitor"][..], &["bridge"][..]] {
        let (ok, _, stderr) = finish(integral(&dir.socket(), args).spawn().expect("run integral"));
        assert!(!ok, "{args:?} ran");
        assert!(
            stderr.contains("--service does not apply"),
            "{args:?}: {stderr}"
        );
    }
}

/// `expansion load` asks the service for the load, which sends it and
/// answers when the device is done.
#[test]
fn an_expansion_load_is_run_by_the_service() {
    use integral_core::address::{Address, DataSize};
    use integral_core::device_spec::INTEGRA7;
    use integral_core::expansion_load::{self, LOAD_DONE_ADDR, LOAD_STARTED_ADDR};
    use integral_core::expansion_slots::{self, ExpansionSlots, SLOT_READ_ADDR};

    let fixture = Fixture::start("expansion");
    let child = fixture
        .integral(&["expansion", "load", "--device-id", "17", "--yes", "A=OFF"])
        .spawn()
        .expect("run integral");

    let current = ExpansionSlots::from_values(&[0x03, 0x05, 0x07, 0x08]).expect("slots");
    fixture.wait_sent(&expansion_slots::build_read_request(DEVICE_ID));
    let says = |address: &Address, data: &[u8]| {
        fixture
            .link
            .device_says(&sysex::build_dt1(DEVICE_ID, address, data));
    };
    says(&SLOT_READ_ADDR, &current.values());
    let bank_size = DataSize::from_value(2).expect("size");
    for part_index in 0..INTEGRA7.part_count {
        let at = params::part_address(part_index, part::TONE_BANK_MSB);
        fixture.wait_sent(&sysex::build_rq1(DEVICE_ID, &at, &bank_size));
        says(&at, &[87, 64]);
    }

    let target = current.with(0, None);
    fixture.wait_sent(&expansion_load::build_load_request(DEVICE_ID, &target));
    // The service's own load, not a raw message passed through: it holds
    // every client's SysEx until the device is done.
    let (tx, rx) = std::sync::mpsc::channel();
    let handle = fixture.running.as_ref().expect("running").handle();
    handle.run(move |service| {
        let _ = tx.send(service.device().expansion_load_target().copied());
    });
    assert_eq!(rx.recv_timeout(WAIT).expect("ran"), Some(target));
    says(&LOAD_STARTED_ADDR, &[0]);
    says(&LOAD_DONE_ADDR, &[0]);

    let (ok, stdout, stderr) = finish(child);
    assert!(ok, "{stderr}");
    assert!(
        stdout.contains("A OFF  B SRX-05  C SRX-07  D SRX-08"),
        "{stdout}"
    );
}

/// A bare `--service` takes no value: the subcommand after it is the
/// command, not a socket path.
#[test]
fn a_bare_service_option_leaves_the_command_alone() {
    let output = Command::new(env!("CARGO_BIN_EXE_integral"))
        .args(["--service", "wave", "list", "INT"])
        .output()
        .expect("run integral");
    assert!(output.status.success(), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stdout).starts_with("0001"));
}

/// The service drops what is pending when its link goes down, without
/// rejecting it: the CLI must not wait out its timeout.
#[test]
fn a_link_that_goes_down_mid_read_fails_the_read() {
    let fixture = Fixture::start("linkdown");
    let started = Instant::now();
    let child = fixture
        .integral(&[
            "raw-read",
            "--device-id",
            "17",
            "--timeout",
            "30",
            "18000000",
            "16",
        ])
        .spawn()
        .expect("run integral");

    let size = integral_core::address::DataSize::from_value(16).expect("size");
    fixture.wait_sent(&sysex::build_rq1(DEVICE_ID, &STUDIO_SET_NAME, &size));
    fixture.link.bring_down();

    let (ok, _, stderr) = finish(child);
    assert!(!ok, "the read succeeded with the link down");
    assert!(started.elapsed() < Duration::from_secs(20), "{stderr}");
}

/// Answer the reads of Part `part`'s (0-based) settings, as `part show` and
/// `part set` make them: the Part dump with its Key Range Upper at `upper`
/// and Coarse Tune at +12, the rest of the Part block, and every Voice
/// Reserve.
fn answer_part_settings_reads(fixture: &Fixture, part: u8, upper: u8) {
    use integral_core::mixer_load::MixerBlock;
    use integral_core::part_settings::PartSetting;

    for block in [
        MixerBlock::Part(part),
        MixerBlock::PartSettings(part),
        MixerBlock::VoiceReserve,
    ] {
        let (at, size) = block.request().expect("a request");
        fixture.wait_sent(&sysex::build_rq1(DEVICE_ID, &at, &size));
        let mut data = vec![0; size.value() as usize];
        if block == MixerBlock::Part(part) {
            data[usize::from(PartSetting::KeyRangeUpper.offset())] = upper;
            data[usize::from(PartSetting::CoarseTune.offset())] = 76;
        }
        fixture
            .link
            .device_says(&sysex::build_dt1(DEVICE_ID, &at, &data));
    }
}

#[test]
fn part_show_reads_the_part_through_the_service() {
    let fixture = Fixture::start("partshow");
    let child = fixture
        .integral(&["part", "show", "16", "--device-id", "17"])
        .spawn()
        .expect("run integral");
    answer_part_settings_reads(&fixture, 15, 100);

    let (ok, stdout, stderr) = finish(child);
    assert!(ok, "{stderr}");
    assert!(stdout.starts_with("Part 16\n"), "{stdout}");
    let row = |name: &str| {
        stdout
            .lines()
            .find(|l| l.split_whitespace().next() == Some(name))
            .and_then(|l| l.split_whitespace().nth(1))
            .map(str::to_string)
    };
    assert_eq!(row("key-upper").as_deref(), Some("E7"));
    assert_eq!(row("coarse-tune").as_deref(), Some("+12"));
}

/// `part set` reads the Part first, so a Lower is held at its Upper.
#[test]
fn part_set_writes_the_field_through_the_service() {
    use integral_core::part_settings::PartSetting;

    let fixture = Fixture::start("partset");
    let child = fixture
        .integral(&["part", "set", "16", "key-lower", "C4", "--device-id", "17"])
        .spawn()
        .expect("run integral");
    answer_part_settings_reads(&fixture, 15, 48);

    let (ok, stdout, stderr) = finish(child);
    assert!(ok, "{stderr}");
    assert_eq!(stdout, "Part 16 key-lower: C3\n");
    assert!(stderr.contains("key-lower held at C3"), "{stderr}");
    let at = PartSetting::KeyRangeLower.address(15).expect("Part 16");
    fixture.wait_sent(&sysex::build_dt1(DEVICE_ID, &at, &[48]));
}

/// A negative value is the value, not an option.
#[test]
fn part_set_takes_a_negative_value() {
    use integral_core::part_settings::PartSetting;

    let fixture = Fixture::start("partneg");
    let child = fixture
        .integral(&[
            "part",
            "set",
            "16",
            "coarse-tune",
            "-48",
            "--device-id",
            "17",
        ])
        .spawn()
        .expect("run integral");
    answer_part_settings_reads(&fixture, 15, 127);

    let (ok, stdout, stderr) = finish(child);
    assert!(ok, "{stderr}");
    assert_eq!(stdout, "Part 16 coarse-tune: -48\n");
    let at = PartSetting::CoarseTune.address(15).expect("Part 16");
    fixture.wait_sent(&sysex::build_dt1(DEVICE_ID, &at, &[16]));
}

#[test]
fn part_set_refuses_an_unknown_field_before_connecting() {
    let output = Command::new(env!("CARGO_BIN_EXE_integral"))
        .args([
            "--service=/nonexistent/socket",
            "part",
            "set",
            "1",
            "level",
            "100",
        ])
        .output()
        .expect("run integral");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown Part setting `level`"), "{stderr}");
}

#[test]
fn part_takes_only_the_devices_parts() {
    let output = Command::new(env!("CARGO_BIN_EXE_integral"))
        .args(["part", "show", "17"])
        .output()
        .expect("run integral");
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("1..=16"),
        "{output:?}"
    );
}

#[test]
fn system_show_reads_system_common_through_the_service() {
    use integral_core::address::SYSTEM_COMMON;
    use integral_core::system::{SYSTEM_COMMON_LEN, SYSTEM_COMMON_SIZE};

    let fixture = Fixture::start("sysshow");
    let child = fixture
        .integral(&["system", "show", "--device-id", CLI_DEVICE_ID])
        .spawn()
        .expect("run integral");

    fixture.wait_sent(&sysex::build_rq1(
        DEVICE_ID,
        &SYSTEM_COMMON,
        &SYSTEM_COMMON_SIZE,
    ));
    let mut data = [0u8; SYSTEM_COMMON_LEN];
    data[0x00..0x04].copy_from_slice(&[0, 4, 0, 0]); // 440.00 Hz
    data[0x04] = 61; // Key Shift -3
    data[0x26..0x28].copy_from_slice(&[7, 8]); // 120
    fixture
        .link
        .device_says(&sysex::build_dt1(DEVICE_ID, &SYSTEM_COMMON, &data));

    let (ok, stdout, stderr) = finish(child);
    assert!(ok, "{stderr}");
    assert!(stdout.contains("master-tune "), "{stdout}");
    assert!(stdout.contains("440.00 Hz"), "{stdout}");
    assert!(stdout.contains("-3"), "{stdout}");
    assert!(stdout.contains("usb-audio-sampling-rate"), "{stdout}");
}

#[test]
fn system_set_writes_the_field_and_says_it_lasts_until_power_off() {
    use integral_core::system::{SystemField, VOLATILE_NOTICE};

    let fixture = Fixture::start("sysset");
    let child = fixture
        .integral(&[
            "system",
            "set",
            "--device-id",
            CLI_DEVICE_ID,
            "master-key-shift",
            "-3",
        ])
        .spawn()
        .expect("run integral");
    let (ok, stdout, stderr) = finish(child);
    assert!(ok, "{stderr}");
    assert!(stdout.contains("master-key-shift = -3"), "{stdout}");
    assert!(stdout.contains(VOLATILE_NOTICE), "{stdout}");

    let dt1 = sysex::build_dt1(DEVICE_ID, &SystemField::MasterKeyShift.address(), &[61]);
    fixture.wait_sent(&dt1);
}

#[test]
fn system_set_refuses_an_unknown_field_before_any_send() {
    let fixture = Fixture::start("sysbad");
    let child = fixture
        // No --device-id: had a port opened, its Identity Request would show.
        .integral(&["system", "set", "master-volume", "1"])
        .spawn()
        .expect("run integral");
    let (ok, _, stderr) = finish(child);
    assert!(!ok);
    assert!(
        stderr.contains("unknown System field: master-volume"),
        "{stderr}"
    );
    assert!(
        fixture.link.sent().is_empty(),
        "{:02X?}",
        fixture.link.sent()
    );
}

/// `studio-set show` reads the Tone Control sources with the tempo, then
/// each MIDI channel's Phase Lock, and prints them by field name.
#[test]
fn studio_set_show_reads_through_the_service() {
    use integral_core::device_spec::MIDI_CHANNELS;
    use integral_core::params::{STUDIO_SET_CONTROL_SIZE, TONE_CONTROL_1_SOURCE, studio_set_midi};

    let fixture = Fixture::start("ssshow");
    let child = fixture
        .integral(&["studio-set", "show", "--device-id", "17"])
        .spawn()
        .expect("run integral");

    let says = |address: &integral_core::address::Address, data: &[u8]| {
        fixture
            .link
            .device_says(&sysex::build_dt1(DEVICE_ID, address, data));
    };
    fixture.wait_sent(&sysex::build_rq1(
        DEVICE_ID,
        &TONE_CONTROL_1_SOURCE,
        &STUDIO_SET_CONTROL_SIZE,
    ));
    says(&TONE_CONTROL_1_SOURCE, &[16, 0, 96, 97, 0x06, 0x00]);
    let one = integral_core::address::DataSize::ONE;
    for channel in 0..MIDI_CHANNELS {
        let at = params::studio_set_midi_address(channel, studio_set_midi::PHASE_LOCK);
        fixture.wait_sent(&sysex::build_rq1(DEVICE_ID, &at, &one));
        says(&at, &[u8::from(channel == 2)]);
    }

    let (ok, stdout, stderr) = finish(child);
    assert!(ok, "{stderr}");
    for line in [
        "tone-control-1-source  CC16",
        "tone-control-2-source  OFF",
        "tone-control-3-source  BEND",
        "tempo                  96",
        "phase-lock-1           OFF",
        "phase-lock-3           ON",
    ] {
        assert!(stdout.lines().any(|l| l == line), "{line}: {stdout}");
    }
}

/// `studio-set set` sends the field's DT1, in its display form's raw value.
#[test]
fn studio_set_set_writes_through_the_service() {
    let fixture = Fixture::start("ssset");
    for (args, address, data) in [
        (["tempo", "96"], params::STUDIO_SET_TEMPO, &[0x06, 0x00][..]),
        (
            ["tone-control-2-source", "CC16"],
            params::TONE_CONTROL_1_SOURCE.offset([0, 0, 0, 1]),
            &[16][..],
        ),
        (
            ["phase-lock-3", "ON"],
            params::studio_set_midi_address(2, params::studio_set_midi::PHASE_LOCK),
            &[1][..],
        ),
    ] {
        let child = fixture
            .integral(&["studio-set", "set", "--device-id", "17", args[0], args[1]])
            .spawn()
            .expect("run integral");
        let (ok, _, stderr) = finish(child);
        assert!(ok, "{stderr}");
        fixture.wait_sent(&sysex::build_dt1(DEVICE_ID, &address, data));
    }
}

/// An unknown field fails before anything is sent.
#[test]
fn studio_set_set_refuses_an_unknown_field() {
    let fixture = Fixture::start("ssbad");
    let child = fixture
        .integral(&["studio-set", "set", "--device-id", "17", "tempi", "96"])
        .spawn()
        .expect("run integral");
    let (ok, _, stderr) = finish(child);
    assert!(!ok);
    assert!(
        stderr.contains("unknown Studio Set field `tempi`"),
        "{stderr}"
    );
    assert!(
        fixture.link.sent().is_empty(),
        "{:02X?}",
        fixture.link.sent()
    );
}
