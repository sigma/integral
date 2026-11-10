import type { PluginKind } from "../pkg/integral_wasm.js";
import { deviceSpec } from "./deviceSpec";
import { noteName } from "./pcm-drum/types";
import { TYPE_MISMATCH_NOTICE, type InstanceInfo, type PluginBinding } from "./plugin";
import { acceptsBinding, kitKeys, pluginToneType } from "./pluginPages";
import { useEffect, useState } from "react";
import { ToneSelector } from "./ToneSelector";
import { toneDisplay } from "./toneBanks";
import type { ToneCatalog } from "./toneCatalog";
import type { UseExpansionsResult } from "./useExpansions";
import type { Expansion } from "./expansionCatalog";
import { NotLoadedMarker } from "./NotLoadedMarker";
import type { PartState } from "./types";
import css from "./PluginBar.module.css";

interface Props {
  kind: PluginKind;
  /** What the plugin says about the instance. */
  info: InstanceInfo;
  /** Why the plugin last refused a request, if it did. */
  refusal?: string | null;
  /** Choose the Binding: the plugin takes it if its kind accepts it. */
  onBind: (binding: PluginBinding) => void;
  /** The bound Part as the mixer holds it; `undefined` while unbound or until the mixer is read. */
  part?: PartState;
  /** The tone catalog the picker lists from. */
  catalog: ToneCatalog | null;
  expansions: UseExpansionsResult;
  /** Change a Part's tone, as the browser's top bar does. */
  onChangeTone: (part: number, msb: number, lsb: number, pc: number) => void;
  /** Open the Expansions page with `expansion` staged. */
  onLoadExpansion: (expansion: Expansion) => void;
}

/** The value of a select whose option means "no key": the whole kit. */
const WHOLE_KIT = "";

/** A select's value as a number, or `undefined` for the empty one. */
function numberOf(value: string): number | undefined {
  return value === "" ? undefined : Number(value);
}

/**
 * The plugin editor's bar: the Binding picker of a Tone or Kit plugin (a
 * Part, and for a Kit plugin the whole kit or one key), and the status line
 * with whatever else the instance reports. The Studio Set plugin has one
 * target and no picker.
 */
export function PluginBar({
  kind,
  info,
  refusal,
  onBind,
  part,
  catalog,
  expansions,
  onChangeTone,
  onLoadExpansion,
}: Props) {
  const binding = info.binding ?? {};
  const keys = kitKeys(kind);
  const toneType = pluginToneType(kind);
  // The plugin owns its Part's tone, but only while the Part holds its type:
  // unbound, unread or on another type, it is inert and picks nothing. A
  // Kit plugin bound to a key never selects: the kit is the whole-kit
  // plugin's (ADR 0009). Nor while the project's saved tone is owed a push:
  // the push would select it over the pick.
  const inert = info.notices.some((notice) => notice.notice === TYPE_MISMATCH_NOTICE);
  const keyBound = binding.key !== undefined;
  const picking =
    toneType !== undefined &&
    binding.part !== undefined &&
    !keyBound &&
    part !== undefined &&
    catalog !== null &&
    !inert &&
    !info.recallOwed
      ? { toneType, index: binding.part, part, catalog }
      : null;
  const whyNot = picking
    ? "Choose tone"
    : binding.part === undefined
      ? "Choose a Part first"
      : inert
        ? "The Part holds another tone type"
        : keyBound
          ? "Bound to a key: the kit belongs to the plugin bound to the whole kit"
          : info.recallOwed
            ? "Waiting to recall the project's tone"
            : "Waiting for the device";
  // The Part the picker was opened for. It shows only while that Part is the
  // bound one and the plugin can still pick; any change of either closes it.
  const [openFor, setOpenFor] = useState<number | null>(null);
  const canPick = picking !== null;
  useEffect(() => setOpenFor(null), [canPick, binding.part]);
  const pickerOpen = picking !== null && openFor === picking.index;
  const unloaded = part ? expansions.unloadedExpansionOf(part.toneBankMsb, part.toneBankLsb) : null;
  return (
    <div className={css.bar}>
      {pluginToneType(kind) !== undefined && (
        <div className={css.picker}>
          <label className={css.label}>
            Part
            <select
              className={css.select}
              value={binding.part ?? ""}
              onChange={(e) => {
                const part = numberOf(e.target.value);
                if (part !== undefined) onBind({ part, key: binding.key });
              }}
            >
              {binding.part === undefined && <option value="">Choose a Part</option>}
              {Array.from({ length: deviceSpec().part_count }, (_, part) => (
                <option key={part} value={part} disabled={!acceptsBinding(kind, { part, key: binding.key })}>
                  Part {part + 1}
                </option>
              ))}
            </select>
          </label>
          {keys && (
            <label className={css.label}>
              Target
              <select
                className={css.select}
                value={binding.key ?? WHOLE_KIT}
                disabled={binding.part === undefined}
                onChange={(e) => onBind({ part: binding.part, key: numberOf(e.target.value) })}
              >
                <option value={WHOLE_KIT}>Whole kit</option>
                {Array.from({ length: keys.last - keys.first + 1 }, (_, i) => keys.first + i).map((key) => (
                  <option key={key} value={key}>
                    Key {noteName(key)} ({key})
                  </option>
                ))}
              </select>
            </label>
          )}
        </div>
      )}
      {toneType !== undefined && (
        <button
          type="button"
          className={css.toneButton}
          data-tone-button
          disabled={picking === null}
          onClick={() => picking && setOpenFor(picking.index)}
          title={whyNot}
        >
          <span className={css.toneName}>
            {binding.part === undefined ? "No Part" : part ? toneDisplay(part) : "…"}
          </span>
          {picking && unloaded && <NotLoadedMarker expansion={unloaded} named />}
          <span aria-hidden>&#x25BC;</span>
        </button>
      )}
      {pickerOpen && picking && (
        <ToneSelector
          partIndex={picking.index}
          currentMsb={picking.part.toneBankMsb}
          currentLsb={picking.part.toneBankLsb}
          currentPC={picking.part.tonePC}
          catalog={picking.catalog}
          expansions={expansions}
          toneType={picking.toneType}
          onSelect={(msb, lsb, pc) => {
            onChangeTone(picking.index, msb, lsb, pc);
            setOpenFor(null);
          }}
          onLoadExpansion={(expansion) => {
            setOpenFor(null);
            onLoadExpansion(expansion);
          }}
          onClose={() => setOpenFor(null)}
        />
      )}
      <div className={css.status} role="status">
        <span className={`${css.statusText} ${info.status === "connected" ? css.ok : css.warn}`}>
          {info.statusText}
        </span>
        {info.notices.map((notice) => (
          <span key={notice.notice} className={css.notice} data-notice={notice.notice}>
            {notice.text}
          </span>
        ))}
      </div>
      {refusal && (
        <span className={css.refusal} role="alert">
          {refusal}
        </span>
      )}
    </div>
  );
}
