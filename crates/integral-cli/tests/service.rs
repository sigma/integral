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
    for args in [
        &["monitor"][..],
        &["bridge"][..],
        &["expansion", "load", "ExPCM"][..],
    ] {
        let (ok, _, stderr) = finish(integral(&dir.socket(), args).spawn().expect("run integral"));
        assert!(!ok, "{args:?} ran");
        assert!(
            stderr.contains("--service does not apply"),
            "{args:?}: {stderr}"
        );
    }
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
