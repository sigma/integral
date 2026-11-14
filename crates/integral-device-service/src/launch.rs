//! The entry point both launchers share: the `integral-device-service`
//! binary and `integral bridge`. It opens the link a [`LinkConfig`] names,
//! runs the service, and serves it on the Unix socket and the WebSocket
//! endpoint until the process is killed, whose exit closes the link and
//! removes the ports.

use std::io;
use std::path::PathBuf;

use clap::Args;
use integral_core::device_service::{WEBSOCKET_PORT, default_origins};
use thiserror::Error;

use crate::link_config::{LinkConfig, LinkConfigError, LinkOverrides};
use crate::{
    Config, NOTE_PORT_NAME, Service, SocketServer, StartError, VirtualNotePort, WebSocketServer,
    socket_path_or_default, spawn, stderr_log,
};

/// The service's command-line arguments.
#[derive(Args, Clone, Debug)]
pub struct ServiceArgs {
    /// The link to the INTEGRA-7, over the configuration file's.
    #[command(flatten)]
    pub link: LinkOverrides,
    /// Unix socket the plugins and other clients connect to, in a directory
    /// only you can enter. Defaults to the one the plugins look for: in the
    /// per-user temporary directory, under `Integral`.
    #[arg(long)]
    pub socket: Option<PathBuf>,
    /// TCP port of the WebSocket endpoint the web app connects to, on
    /// 127.0.0.1 only. The web app always uses the default, so change it only
    /// for tests or when the port is taken.
    #[arg(long, default_value_t = WEBSOCKET_PORT)]
    pub ws_port: u16,
    /// An `Origin` allowed to use the WebSocket endpoint, as a browser writes
    /// it (`scheme://host[:port]`, no path). Repeat for several; given at
    /// all, these replace the defaults (the `just dev-web` and `vite preview`
    /// servers, and the app on GitHub Pages: `https://sigma.github.io` admits
    /// every Pages site of that user). A refused origin is logged here.
    #[arg(long = "ws-origin", value_name = "ORIGIN", default_values_t = default_origins())]
    pub ws_origins: Vec<String>,
}

/// Why the service did not start. Each message includes its cause, so none
/// has a `source`: a report walking the chain would print the cause twice.
#[derive(Debug, Error)]
pub enum LaunchError {
    /// The default socket path could not be found.
    #[error("find the default socket path: {0}")]
    SocketPath(io::Error),
    /// The configuration names no link.
    #[error(transparent)]
    Link(#[from] LinkConfigError),
    /// The service did not start.
    #[error("start the Device service: {0}")]
    Start(StartError),
    /// The Unix socket could not be served.
    #[error("listen on {}: {error}", path.display())]
    Listen {
        /// The socket.
        path: PathBuf,
        /// The OS error.
        error: io::Error,
    },
    /// The WebSocket endpoint could not be served.
    #[error("start the WebSocket endpoint on 127.0.0.1:{port}: {error}")]
    WebSocket {
        /// The TCP port.
        port: u16,
        /// The OS error.
        error: io::Error,
    },
}

impl ServiceArgs {
    /// The link to open: `file` with these arguments' flags over it.
    pub fn link_config(&self, file: LinkConfig) -> LinkConfig {
        file.with_overrides(&self.link)
    }

    /// Run the Device service on the link `file` names, with these
    /// arguments' flags over it, until the process is killed.
    ///
    /// # Errors
    ///
    /// [`LaunchError`], when the service cannot start.
    pub fn run(&self, file: LinkConfig) -> Result<(), LaunchError> {
        let path =
            socket_path_or_default(self.socket.as_deref()).map_err(LaunchError::SocketPath)?;
        let link_config = self.link_config(file);
        let link = link_config.open(stderr_log())?;
        let service = Service::new(link, Config::new(link_config.device_id));
        let running =
            spawn(service, Some(Box::new(VirtualNotePort::new()))).map_err(LaunchError::Start)?;
        let server = match SocketServer::bind(&path, running.handle()) {
            Ok(server) => server,
            Err(error) => {
                running.stop();
                return Err(LaunchError::Listen { path, error });
            }
        };
        let web = match WebSocketServer::bind(
            self.ws_port,
            &self.ws_origins,
            running.handle(),
            stderr_log(),
        ) {
            Ok(web) => web,
            Err(error) => {
                drop(server);
                running.stop();
                return Err(LaunchError::WebSocket {
                    port: self.ws_port,
                    error,
                });
            }
        };
        eprintln!(
            "Device service running; notes go to the MIDI port '{NOTE_PORT_NAME}', clients to {}, \
             the web app to ws://{} (origins: {})",
            server.path().display(),
            web.local_addr(),
            self.ws_origins.join(", ")
        );
        running.join();
        Ok(())
    }
}
