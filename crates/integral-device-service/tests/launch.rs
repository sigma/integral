//! Starting the service: what it checks before it opens the link.

#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};

use clap::Parser;
use integral_device_service::{
    LaunchError, LinkConfig, ServiceArgs, SocketServer, ensure_private_dir, socket_path_in,
};

#[derive(Parser)]
struct Cli {
    #[command(flatten)]
    service: ServiceArgs,
}

#[test]
fn a_second_service_stops_before_opening_the_link() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("launch{}", std::process::id()));
    ensure_private_dir(&dir).expect("private temp dir");
    let path: PathBuf = socket_path_in(&dir);
    let _first = SocketServer::claim(&path).expect("the first service claims the path");

    // An rtp link with no port fails as soon as it is opened, so reaching the
    // link would return `LaunchError::Link` instead.
    let cli = Cli::try_parse_from([
        "service",
        "--link",
        "rtp",
        "--socket",
        path.to_str().expect("utf-8 path"),
    ])
    .expect("flags");
    let err = cli
        .service
        .run(LinkConfig::default())
        .expect_err("already running");

    assert!(
        matches!(err, LaunchError::AlreadyRunning { .. }),
        "got {err}"
    );
    assert!(err.to_string().contains(&path.display().to_string()));
    let _ = fs::remove_dir_all(&dir);
}
