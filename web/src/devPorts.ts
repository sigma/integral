/**
 * The ports the dev and preview servers are pinned to (`vite.config.ts`).
 *
 * The Device service admits the pages these serve as origins by default
 * (`integral_core::device_service::default_origins`), so the two must agree:
 * a test compares them with the core's, through `devServerPort` and
 * `previewPort`.
 */

/** The port of `just dev-web`. */
export const DEV_SERVER_PORT = 5181;

/** The port of `vite preview`. */
export const PREVIEW_PORT = 4181;
