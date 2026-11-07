import type { PluginKind } from "../pkg/integral_wasm.js";
import { deviceSpec } from "./deviceSpec";
import { noteName } from "./pcm-drum/types";
import type { InstanceInfo, PluginBinding } from "./plugin";
import { acceptsBinding, kitKeys, pluginToneType } from "./pluginPages";
import css from "./PluginBar.module.css";

interface Props {
  kind: PluginKind;
  /** What the plugin says about the instance. */
  info: InstanceInfo;
  /** Why the plugin last refused a request, if it did. */
  refusal?: string | null;
  /** Choose the Binding: the plugin takes it if its kind accepts it. */
  onBind: (binding: PluginBinding) => void;
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
export function PluginBar({ kind, info, refusal, onBind }: Props) {
  const binding = info.binding ?? {};
  const keys = kitKeys(kind);
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
