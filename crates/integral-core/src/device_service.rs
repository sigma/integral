//! Where a client finds the Device service, for the clients that cannot ask
//! the operating system: the web app, which reaches it over a localhost
//! WebSocket (ADR 0006). The service listens here by default; the browser
//! learns the address from here through `integral-wasm`.

use std::net::Ipv4Addr;

/// The name of the page's Device service port pair: what the port picker
/// shows and the last-port memory keeps, and the port the plugin editor
/// offers its page. The page's `DEVICE_SERVICE_NAME` is checked against it.
pub const PAIR_NAME: &str = "Integral Device Service";

/// The page's `id` for the Device service pair, as [`PAIR_NAME`].
pub const PAIR_ID: &str = "integral-device-service";

/// The default TCP port of the service's WebSocket endpoint.
pub const WEBSOCKET_PORT: u16 = 4707;

/// The address the endpoint listens on: loopback only, so nothing off this
/// machine can reach the device.
pub const WEBSOCKET_HOST: Ipv4Addr = Ipv4Addr::LOCALHOST;

/// The port of the web dev server (`just dev-web`). `web/vite.config.ts`
/// pins it (`strictPort`), and a web test checks the two agree: the origin
/// the service admits is only right while they do.
pub const DEV_SERVER_PORT: u16 = 5181;

/// The port of `vite preview`, pinned in `web/vite.config.ts` like
/// [`DEV_SERVER_PORT`].
pub const PREVIEW_PORT: u16 = 4181;

/// The origin of the app deployed to GitHub Pages. An origin has no path, so
/// this admits every Pages site of that user, not only `/integral/`.
pub const DEPLOYED_ORIGIN: &str = "https://sigma.github.io";

/// The URL of the endpoint on `port`.
pub fn websocket_url(port: u16) -> String {
    format!("ws://{WEBSOCKET_HOST}:{port}")
}

/// The origins the endpoint admits by default: the dev server and the
/// preview server, each as `localhost` and as `127.0.0.1`, and the deployed
/// app ([`DEPLOYED_ORIGIN`]).
pub fn default_origins() -> Vec<String> {
    [DEV_SERVER_PORT, PREVIEW_PORT]
        .into_iter()
        .flat_map(|port| {
            [
                format!("http://localhost:{port}"),
                format!("http://127.0.0.1:{port}"),
            ]
        })
        .chain([DEPLOYED_ORIGIN.to_string()])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_url_names_the_loopback_address_and_the_port() {
        assert_eq!(websocket_url(WEBSOCKET_PORT), "ws://127.0.0.1:4707");
    }

    #[test]
    fn the_default_origins_name_the_servers_and_the_deployed_app() {
        let origins = default_origins();
        assert!(origins.contains(&"http://localhost:5181".to_string()));
        assert!(origins.contains(&"http://127.0.0.1:4181".to_string()));
        assert!(origins.contains(&DEPLOYED_ORIGIN.to_string()));
    }
}
