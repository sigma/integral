/**
 * The Routing page ([#612](https://github.com/sigma/integral/issues/612)): where each source's
 * sound goes, as core's effective routing computes it from the mixer state. One row per source,
 * one column per output, a mark where the row lands. It edits the stage switches only; every
 * other value is a chip linking to the page that edits it.
 */

import { useEffect, type ReactNode } from "react";
import {
  MixerField,
  MixerScopeKind,
  type EffectiveRouting,
  type Route,
  type RoutingSend,
  type RoutingSource,
} from "../pkg/integral_wasm.js";
import { deviceSpec } from "./deviceSpec";
import { getMfxTypeNames } from "./MfxPanel";
import type { PageTab } from "./pluginPages";
import { PowerButton } from "./synth-ui/PowerButton";
import type { MixerState } from "./types";
import type { UseMixerResult } from "./useMixer";
import css from "./RoutingPage.module.css";

interface Props {
  mixer: UseMixerResult;
  /** Open the page a chip links to. */
  onOpen: (page: PageTab) => void;
}

/** A column of the map: its heading, and whether a route is marked in it. */
interface Column {
  label: string;
  marks: (route: Route) => boolean;
}

/** A source's identity, as a string (core serializes its fields in one order). */
function sourceKey(source: RoutingSource): string {
  return JSON.stringify(source);
}

/** Whether `a` and `b` are one source. */
function same(a: RoutingSource, b: RoutingSource): boolean {
  return sourceKey(a) === sourceKey(b);
}

/** A source's name, as its row is headed. */
function sourceName(source: RoutingSource): string {
  switch (source.kind) {
    case "part":
      return `Part ${source.part + 1}`;
    case "extPart":
      return "Ext Part";
    case "chorus":
      return "Chorus";
    case "reverb":
      return "Reverb";
    case "compEq":
      return `Comp+EQ ${source.unit + 1}`;
  }
}

/** What a row's heading adds under the name: a Part's tone, a unit's Part. */
function sourceDetail(source: RoutingSource, state: MixerState): string {
  if (source.kind === "part") return state.parts[source.part]?.toneName ?? "";
  if (source.kind === "compEq") return `Part ${source.part + 1}`;
  return "";
}

/** The Part whose settings a source's chips edit, if it has one. */
function partOf(source: RoutingSource): number | undefined {
  return source.kind === "part" || source.kind === "compEq" ? source.part : undefined;
}

/** The columns while Motional Surround is off: one per output, by the spec's names. */
function outputColumns(): Column[] {
  return deviceSpec().output_assigns.map((name, output) => ({
    // The first pair is the 2-channel mix (`docs/research/audio-output-routing.md`).
    label: output === 0 ? `${name} (MIX)` : name,
    marks: (route) => route.landsOn.some((l) => l.kind === "output" && l.output === output),
  }));
}

/** The columns while Motional Surround is on: each jack's channels, as core names them. */
function surroundColumns(routing: EffectiveRouting): Column[] {
  return routing.jacks.flatMap((jack) =>
    [jack.left, jack.right].map((sources, side) => ({
      label: jack.surroundChannels?.[side] ?? "",
      marks: (route: Route) => sources.some((s) => same(s, route.source)),
    })),
  );
}

/** The other sources on the jacks `source` comes out of: they sum with it. */
function summingWith(routing: EffectiveRouting, source: RoutingSource): RoutingSource[] {
  const others: RoutingSource[] = [];
  for (const jack of routing.jacks) {
    for (const side of [jack.left, jack.right]) {
      if (!side.some((s) => same(s, source))) continue;
      for (const s of side) {
        if (!same(s, source) && !others.some((o) => same(o, s))) others.push(s);
      }
    }
  }
  return others;
}

/** The name of MFX Type `type`; THRU (type 0) in capitals, as the device shows it. */
function mfxName(type: number): string {
  const name = getMfxTypeNames()[type] ?? `#${type}`;
  return type === 0 ? name.toUpperCase() : name;
}

/** The page: the stage switches, the Surround banner, then the map. */
export function RoutingPage({ mixer, onOpen }: Props) {
  const { state, sync } = mixer;
  // What other pages write without a mixer sync (a tone editor's MFX Type) shows on opening.
  useEffect(() => sync(), [sync]);
  const routing = state.routing;
  if (state.loading || !routing) {
    return (
      <div className={css.page} role="region" aria-label="Routing">
        <div className={css.loading}>Loading mixer state from device...</div>
      </div>
    );
  }

  const spec = deviceSpec();
  const setSwitch = (kind: MixerScopeKind, index: number, field: MixerField, on: boolean) =>
    mixer.setMixerField(kind, index, field, on ? 1 : 0);
  /** Open `page`, with `source`'s Part selected if it has one. */
  const openEditor = (page: PageTab, source?: RoutingSource) => {
    const part = source && partOf(source);
    if (part !== undefined) mixer.selectPart(part);
    onOpen(page);
  };

  const stages: { label: string; on: boolean; toggle: () => void }[] = [
    {
      label: "Chorus",
      on: state.chorus.enabled,
      toggle: () => setSwitch(MixerScopeKind.Common, 0, MixerField.ChorusSwitch, !state.chorus.enabled),
    },
    {
      label: "Reverb",
      on: state.reverb.enabled,
      toggle: () => setSwitch(MixerScopeKind.Common, 0, MixerField.ReverbSwitch, !state.reverb.enabled),
    },
    {
      label: "Master EQ",
      on: state.masterEq.enabled,
      toggle: () => setSwitch(MixerScopeKind.Master, 0, MixerField.EqSwitch, !state.masterEq.enabled),
    },
    {
      label: "Drum Comp+EQ",
      on: state.drumCompEq.enabled,
      toggle: () => setSwitch(MixerScopeKind.Common, 0, MixerField.DrumCompEqSwitch, !state.drumCompEq.enabled),
    },
    {
      label: "Motional Surround",
      on: state.surround.enabled,
      toggle: () => setSwitch(MixerScopeKind.Common, 0, MixerField.SurroundSwitch, !state.surround.enabled),
    },
  ];

  const columns = routing.surround ? surroundColumns(routing) : outputColumns();

  /** A send chip, dim while the effect it feeds doesn't play. */
  const sendChip = (label: string, send: RoutingSend | undefined, source: RoutingSource) =>
    send && (
      <Chip
        dim={!send.live}
        title={send.live ? `${label} send` : `${label} send: the effect doesn't play`}
        onClick={() => openEditor("mixer", source)}
      >
        {label} {send.level}
      </Chip>
    );

  /** A row's stage chips. */
  const chips = (route: Route): ReactNode => {
    const { source } = route;
    const part = partOf(source);
    return (
      <>
        {route.mfx &&
          (route.mfx.kind === "skipped" ? (
            <Chip skipped title="The Comp+EQ unit skips the Part's MFX">
              MFX
            </Chip>
          ) : (
            <Chip title="Edit the tone's MFX" onClick={() => openEditor("toneEdit", source)}>
              MFX {route.mfx.mfxType === undefined ? "…" : mfxName(route.mfx.mfxType)}
            </Chip>
          ))}
        {route.partEq &&
          (route.partEq === "skipped" ? (
            <Chip skipped title="The Comp+EQ unit skips the Part EQ">
              EQ
            </Chip>
          ) : (
            <Chip
              pressed={route.partEq === "on"}
              title={`Part EQ ${route.partEq}`}
              onClick={
                part === undefined
                  ? undefined
                  : () => setSwitch(MixerScopeKind.Part, part, MixerField.EqSwitch, route.partEq !== "on")
              }
            >
              EQ
            </Chip>
          ))}
        {source.kind === "compEq" && (
          <Chip title="Edit the unit in the Mixer" onClick={() => openEditor("mixer", source)}>
            COMP+EQ
          </Chip>
        )}
        {sendChip("CHO", route.chorusSend, source)}
        {sendChip("REV", route.reverbSend, source)}
        {route.feedsReverb && (
          <Chip title="The Chorus feeds the Reverb" onClick={() => openEditor("mixer")}>
            → REV
          </Chip>
        )}
      </>
    );
  };

  /** A row's Output Assign, as set: struck through while Motional Surround overrides it. */
  const assign = (route: Route): ReactNode => {
    if (!route.assign) return null;
    const name =
      route.assign.kind === "part" ? spec.comp_eq_output_assigns[0] : spec.output_assigns[route.assign.output];
    return (
      <button
        type="button"
        data-assign
        data-overridden={routing.surround || undefined}
        className={`${css.assign} ${routing.surround ? css.overridden : ""}`}
        title={
          routing.surround
            ? "Motional Surround overrides this Output Assign: open the Surround page"
            : "Edit in the Mixer"
        }
        onClick={() => (routing.surround ? onOpen("surround") : openEditor("mixer", route.source))}
      >
        {name}
      </button>
    );
  };

  /** A row's mark in a column; under the 2-channel routing, it names what it sums with. */
  const mark = (route: Route): ReactNode => {
    const others = routing.surround ? [] : summingWith(routing, route.source);
    return (
      <span
        data-mark
        data-sums={others.length > 0 || undefined}
        className={`${css.mark} ${others.length > 0 ? css.sums : ""}`}
        title={others.length > 0 ? `Sums with ${others.map(sourceName).join(", ")}` : undefined}
      />
    );
  };

  return (
    <div className={css.page} role="region" aria-label="Routing">
      <div className={css.stages} data-stage role="group" aria-label="Stages">
        {stages.map((stage) => (
          <PowerButton key={stage.label} className={css.stage} isOn={stage.on} onToggle={stage.toggle}>
            <span>{stage.label}</span>
          </PowerButton>
        ))}
      </div>
      {routing.surround && (
        <div className={css.banner} role="status">
          Motional Surround is on: it overrides every Output Assign. The jacks carry the 2-channel mix and the 5.1
          layout below.
          <button type="button" className={css.bannerLink} onClick={() => onOpen("surround")}>
            Surround page
          </button>
        </div>
      )}
      <table className={css.map}>
        <thead>
          <tr>
            <th>Source</th>
            <th>Stages</th>
            <th>Assign</th>
            {columns.map((column) => (
              <th key={column.label} data-column className={css.columnHead}>
                {column.label}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {routing.routes.map((route) => (
            <tr key={sourceKey(route.source)} className={route.landsOn.length === 0 ? css.silent : ""}>
              <th className={css.source} scope="row">
                <span data-source-name>{sourceName(route.source)}</span>
                <span className={css.detail}>{sourceDetail(route.source, state)}</span>
              </th>
              <td className={css.chips}>{chips(route)}</td>
              <td>{assign(route)}</td>
              {columns.map((column, i) => (
                <td key={i} data-column className={css.cell}>
                  {column.marks(route) && mark(route)}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

interface ChipProps {
  children: ReactNode;
  title: string;
  /** Opens its editor, or flips its stage; a chip without one only reads. */
  onClick?: () => void;
  /** A switch chip's state. */
  pressed?: boolean;
  /** The stage is skipped on this path. */
  skipped?: boolean;
  /** The stage is set but has no effect (a send into an effect that doesn't play). */
  dim?: boolean;
}

/** One stage of a row: a link to its editor, or its switch. */
function Chip({ children, title, onClick, pressed, skipped = false, dim = false }: ChipProps) {
  const className = `${css.chip} ${skipped ? css.skipped : ""} ${dim ? css.dim : ""} ${pressed === false ? css.off : ""}`;
  return onClick ? (
    <button type="button" data-chip className={className} title={title} aria-pressed={pressed} onClick={onClick}>
      {children}
    </button>
  ) : (
    <span data-chip data-skipped={skipped || undefined} className={className} title={title}>
      {children}
    </span>
  );
}
