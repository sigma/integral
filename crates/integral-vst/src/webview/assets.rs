//! The embedded web build, served over the editor's custom protocol.
//!
//! `build.rs` embeds `web/dist-plugin`; [`serve`] maps a request path to
//! the file's bytes and MIME type. WASM must be `application/wasm` or
//! `WebAssembly.instantiateStreaming` refuses it, and the page's CSP must
//! allow `'wasm-unsafe-eval'` to compile it.

include!(concat!(env!("OUT_DIR"), "/web_assets.rs"));

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

/// Shown when the plugin was built without `web/dist-plugin`.
const NOT_BUILT: &[u8] = b"<!doctype html><meta charset=utf-8>\
<body style=\"background:#0f0f1a;color:#e0e0e0;font-family:sans-serif;padding:2em\">\
<h1>Integral</h1><p>This plugin was built without its web UI. \
Run <code>just build-web-plugin</code>, then rebuild the plugin.</p>";

/// A response to one request for an embedded file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Served {
    /// HTTP status: 200 or 404.
    pub status: u16,
    /// `Content-Type` of `body`.
    pub mime: &'static str,
    /// The file's bytes; empty on 404.
    pub body: &'static [u8],
}

impl Served {
    const NOT_FOUND: Served = Served {
        status: 404,
        mime: "text/plain",
        body: b"",
    };
}

/// The response for the request path `path` (the URI's path, no query),
/// from `assets` (sorted by path). `/` is `index.html`. When `assets` is
/// empty, `index.html` is a page saying how to build the UI.
pub(crate) fn serve_from(assets: &[(&str, &'static [u8])], path: &str) -> Served {
    let rel = match path.strip_prefix('/').unwrap_or(path) {
        "" => "index.html",
        rel => rel,
    };
    // The table holds only the build, so `..` can't escape it; refusing it
    // keeps an odd path from aliasing a real one.
    if rel
        .split('/')
        .any(|seg| seg.is_empty() || seg == "." || seg == "..")
    {
        return Served::NOT_FOUND;
    }
    match assets.binary_search_by(|(p, _)| p.cmp(&rel)) {
        Ok(i) => Served {
            status: 200,
            mime: mime(rel),
            body: assets[i].1,
        },
        Err(_) if assets.is_empty() && rel == "index.html" => Served {
            status: 200,
            mime: mime(rel),
            body: NOT_BUILT,
        },
        Err(_) => Served::NOT_FOUND,
    }
}

/// The response for `path` from the embedded build.
pub(crate) fn serve(path: &str) -> Served {
    serve_from(ASSETS, path)
}

/// The MIME type of `path`, by extension.
pub(crate) fn mime(path: &str) -> &'static str {
    let ext = path.rsplit_once('.').map_or("", |(_, ext)| ext);
    match ext.to_ascii_lowercase().as_str() {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "wasm" => "application/wasm",
        "json" => "application/json",
        "webmanifest" => "application/manifest+json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUILD: &[(&str, &[u8])] = &[
        ("assets/index-abc.js", b"js"),
        ("assets/integral_wasm_bg-def.wasm", b"\0asm"),
        ("assets/style.css", b"css"),
        ("icon.svg", b"<svg/>"),
        ("index.html", b"<html>"),
    ];

    #[test]
    fn root_is_index_html() {
        let served = serve_from(BUILD, "/");
        assert_eq!(served.status, 200);
        assert_eq!(served.body, b"<html>");
        assert_eq!(served.mime, "text/html; charset=utf-8");
    }

    #[test]
    fn serves_each_file_with_its_mime_type() {
        for (path, body, mime) in [
            (
                "/assets/index-abc.js",
                &b"js"[..],
                "text/javascript; charset=utf-8",
            ),
            (
                "/assets/integral_wasm_bg-def.wasm",
                b"\0asm",
                "application/wasm",
            ),
            ("/assets/style.css", b"css", "text/css; charset=utf-8"),
            ("/icon.svg", b"<svg/>", "image/svg+xml"),
        ] {
            let served = serve_from(BUILD, path);
            assert_eq!(
                served,
                Served {
                    status: 200,
                    mime,
                    body
                },
                "{path}"
            );
        }
    }

    #[test]
    fn unknown_paths_are_not_found() {
        for path in [
            "/missing.js",
            "/assets",
            "/assets/",
            "/../index.html",
            "/assets/../index.html",
            "//index.html",
        ] {
            assert_eq!(serve_from(BUILD, path).status, 404, "{path}");
        }
    }

    #[test]
    fn without_a_build_index_says_how_to_build_it() {
        let served = serve_from(&[], "/");
        assert_eq!(served.status, 200);
        assert_eq!(served.mime, "text/html; charset=utf-8");
        assert!(
            std::str::from_utf8(served.body)
                .unwrap()
                .contains("just build-web-plugin")
        );
        assert_eq!(serve_from(&[], "/assets/x.js").status, 404);
    }

    #[test]
    fn mime_types_by_extension() {
        assert_eq!(mime("a.WASM"), "application/wasm");
        assert_eq!(mime("manifest.webmanifest"), "application/manifest+json");
        assert_eq!(mime("noext"), "application/octet-stream");
    }

    #[test]
    fn the_embedded_table_is_sorted() {
        assert!(ASSETS.windows(2).all(|w| w[0].0 < w[1].0));
    }
}
