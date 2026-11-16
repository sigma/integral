//! The embedded web build, served over the editor's custom protocol.
//!
//! `integral-web-assets` embeds `web/dist-plugin` and maps a request path to
//! the file's bytes and MIME type ([`serve`]); the page's CSP must allow
//! `'wasm-unsafe-eval'` to compile the WASM.

// Only the macOS editor serves pages; elsewhere the re-export would be unused.
#[cfg(target_os = "macos")]
pub(crate) use integral_web_assets::serve;

/// The scheme the page is served from (`integral://localhost/`).
pub(crate) const SCHEME: &str = "integral";

/// The page's URL in the webview.
pub(crate) const START_URL: &str = "integral://localhost/";

/// The page's Content Security Policy.
///
/// Everything comes from the embedded build (`'self'`, or the scheme where
/// the engine doesn't count a custom scheme as `'self'`). WASM needs
/// `'wasm-unsafe-eval'`; React sets inline `style` attributes, and the
/// favicon and Wave thumbnails may be `data:` URLs.
pub(crate) const CSP: &str = "default-src 'self' integral:; \
     script-src 'self' integral: 'wasm-unsafe-eval'; \
     style-src 'self' integral: 'unsafe-inline'; \
     img-src 'self' integral: data:; \
     connect-src 'self' integral:; \
     object-src 'none'; base-uri 'none'";
