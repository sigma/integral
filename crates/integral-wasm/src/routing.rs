//! WASM binding for the effective routing
//! ([`integral_core::routing`]): where each source's sound goes.

use integral_core::routing;
use wasm_bindgen::prelude::*;

use crate::WasmDeviceState;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
/** A source of sound, a row of the routing map (Parts 0-based). */
export type RoutingSource =
  | { kind: "part"; part: number }
  | { kind: "extPart" }
  | { kind: "chorus" }
  | { kind: "reverb" }
  | { kind: "compEq"; unit: number; part: number };
/** An output: the index of its name in the spec's `outputAssigns` (A-D, then 1-8). */
export type RoutingOutput = number;
/** The Part MFX on a path: its type (0 = THRU; absent while unread), or skipped by a Comp+EQ unit. */
export type MfxStage = { kind: "applied"; mfxType?: number } | { kind: "skipped" };
/** The Part EQ on a path. */
export type EqStage = "on" | "off" | "skipped";
/** A send into the Chorus or Reverb; `live` while that effect plays. */
export interface RoutingSend { level: number; live: boolean }
/** An Output Assign as set: an output, or a Comp+EQ unit's PART. */
export type RoutingAssign = { kind: "output"; output: RoutingOutput } | { kind: "part" };
/** Where a source comes out: an output, or the Motional Surround mix on every jack. */
export type RoutingLanding = { kind: "output"; output: RoutingOutput } | { kind: "surround" };
/** One source's path: its stages, and where it lands. */
export interface Route {
  source: RoutingSource;
  mfx?: MfxStage;
  partEq?: EqStage;
  chorusSend?: RoutingSend;
  reverbSend?: RoutingSend;
  /** The Chorus on Output Select REV or MAIN+REV, while both play. */
  feedsReverb: boolean;
  /** The Output Assign as set; Motional Surround ON overrides it. */
  assign?: RoutingAssign;
  /** Empty when it comes out nowhere. */
  landsOn: RoutingLanding[];
}
/** A jack pair (index in the spec's `outputAssignNames`), and the sources summing on each side. */
export interface RoutingJack {
  pair: number;
  /** Its left and right channels while Motional Surround is on. */
  surroundChannels?: [string, string];
  left: RoutingSource[];
  right: RoutingSource[];
}
/** The effective routing of the mixer state. */
export interface EffectiveRouting {
  /** Motional Surround is on, overriding every Output Assign. */
  surround: boolean;
  /** The Parts, the Ext Part, the Chorus, the Reverb, then the Comp+EQ units in use. */
  routes: Route[];
  /** One per jack pair, A (MIX) first. */
  jacks: RoutingJack[];
}
"#;

#[wasm_bindgen]
impl WasmDeviceState {
    /// The effective routing of the mixer state; see
    /// [`routing::effective_routing`].
    #[wasm_bindgen(js_name = effectiveRouting, unchecked_return_type = "EffectiveRouting")]
    pub fn effective_routing(&self) -> Result<JsValue, JsError> {
        let routing = routing::effective_routing(self.inner.state());
        serde_wasm_bindgen::to_value(&routing).map_err(|e| JsError::new(&e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use integral_core::routing::effective_routing;
    use integral_core::state::MixerState;
    use serde_json::json;

    #[test]
    fn the_routing_crosses_in_the_shape_its_typescript_types_say() {
        // serde_json writes an absent field as `null`; serde_wasm_bindgen
        // leaves it `undefined`, as the optional TypeScript fields say.
        let mut state = MixerState::default();
        state.parts[9].tone_bank_msb = 88; // an SN Drum Kit
        state.parts[9].mfx_type = Some(12);
        state.drum_comp_eq.enabled = true;
        state.drum_comp_eq.output_assigns = [3, 0, 0, 0, 0, 0];
        let routing = serde_json::to_value(effective_routing(&state)).expect("serializes");

        assert_eq!(routing["surround"], json!(false));
        assert_eq!(
            routing["routes"][9],
            json!({
                "source": { "kind": "part", "part": 9 },
                "mfx": { "kind": "applied", "mfxType": 12 },
                "partEq": "on",
                "chorusSend": { "level": 0, "live": false },
                "reverbSend": { "level": 0, "live": false },
                "feedsReverb": false,
                "assign": { "kind": "output", "output": 0 },
                "landsOn": [{ "kind": "output", "output": 0 }],
            })
        );
        assert_eq!(
            routing["routes"][16]["source"],
            json!({ "kind": "extPart" })
        );
        assert_eq!(routing["routes"][16]["assign"], json!(null));
        let unit = &routing["routes"][19];
        assert_eq!(
            unit["source"],
            json!({ "kind": "compEq", "unit": 0, "part": 9 })
        );
        assert_eq!(unit["mfx"], json!({ "kind": "skipped" }));
        assert_eq!(unit["partEq"], json!("skipped"));
        assert_eq!(routing["routes"][20]["assign"], json!({ "kind": "part" }));
        assert_eq!(routing["jacks"][0]["pair"], json!(0));
        assert_eq!(routing["jacks"][0]["surroundChannels"], json!(null));

        state.surround.enabled = true;
        let routing = serde_json::to_value(effective_routing(&state)).expect("serializes");
        assert_eq!(
            routing["routes"][0]["landsOn"],
            json!([{ "kind": "surround" }])
        );
        assert_eq!(
            routing["jacks"][3]["surroundChannels"],
            json!(["Rs", "LFE"])
        );
    }
}
