/**
 * A mixer FX column on the Grid (#392): the Chorus's or the Reverb's
 * parameter Section where the other strips have their EQ, its strip (a
 * variant of `FX_STRIP_SET`) and the plate under it. The strip writes
 * through `setMixerField` in the Common Scope; the parameters write their
 * block's nibblized slots.
 */

import { useMemo } from "react";
import { DisplayKind, MixerField, MixerScopeKind } from "../pkg/integral_wasm.js";
import { CategoryPlate, VerticalName, type SetMixerField } from "./ChannelStrip";
import { stripIcon } from "./categoryIcons";
import { useFxBind } from "./fxBinding";
import { CHORUS_SET, GridSection, REVERB_SET, variantLayout, type Binding } from "./layouts";
import { FX_STRIP_SET, FX_STRIP_FIELDS, type FxStripKind } from "./layouts/mixer/strip";
import { mixerBinding, paramFormatter, paramMeta } from "./paramDefaults";
import type { FxState } from "./types";

/** Each FX column's parameter Section, by block. */
const SETS = { chorus: CHORUS_SET, reverb: REVERB_SET } as const;

/** Props of {@link FxParams}. */
interface FxParamsProps {
  /** Which FX block: its params and their layout come from core by type. */
  block: FxStripKind;
  /** The block's state. */
  fx: FxState;
  /** Writes one of the block's nibblized parameter slots. */
  onNibParam: (slot: number, value: number) => void;
}

/** An FX column's parameter Section, dimmed while its switch is off. */
export function FxParams({ block, fx, onNibParam }: FxParamsProps) {
  // One footprint for every type, so nothing moves when the type changes.
  const layout = useMemo(() => variantLayout(SETS[block], fx.type), [block, fx.type]);
  const bind = useFxBind(block, fx.type, fx.params, onNibParam);
  // Some types' knobs carry a sync toggle: every type's knobs keep the dial they leave room for.
  return <GridSection layout={layout} bind={bind} isActive={() => fx.enabled} reserveAddon />;
}

/** The `FxState` member holding each FX strip field's value. */
function fxValue(block: FxStripKind, fx: FxState, field: MixerField): number {
  const { switch: sw, type, level, outputs } = FX_STRIP_FIELDS[block];
  if (field === sw) return Number(fx.enabled);
  if (field === type) return fx.type;
  if (field === level) return fx.level;
  // The Chorus keeps its Output Assign apart from its Output Select; the Reverb's one output is its assign.
  if (field === MixerField.ChorusOutputAssign) return fx.outputAssign;
  if (outputs.includes(field)) return fx.output;
  throw new Error(`${MixerField[field]} is not on the ${block} strip`);
}

/** Props of {@link FxStrip}. */
interface FxStripProps {
  /** Which FX. */
  block: FxStripKind;
  /** The block's state. */
  fx: FxState;
  /** Writes a mixer field; the strip's are in the Common Scope. */
  setField: SetMixerField;
}

/** The strip Section's accent: the family of its level fader, as on every strip. */
const stripFamily = () => paramMeta(MixerField.PartLevel).family;

/** An FX column's strip: switch, type, outputs and level, the type's name up the fader. */
export function FxStrip({ block, fx, setField }: FxStripProps) {
  const layout = useMemo(() => variantLayout(FX_STRIP_SET, block), [block]);
  const { switch: sw, type, level } = FX_STRIP_FIELDS[block];
  const bind = (field: MixerField): Binding => ({
    ...mixerBinding(field, fxValue(block, fx, field), (v) => setField(MixerScopeKind.Common, 0, field, v)),
    // The switch reads FX, on and off alike: its LED shows the state.
    ...(field === sw ? { options: [{ value: 0, label: "FX" }, { value: 1, label: "FX" }] } : {}),
  });
  return (
    <GridSection
      layout={layout}
      bind={bind}
      scope={{ fx: FX_STRIP_SET.keys.indexOf(block) }}
      family={stripFamily()}
      // The level greys out with the switch, as the old FX fader did.
      isActive={(field) => fx.enabled || field !== level}
      renderDisplay={(display) =>
        display === DisplayKind.StripName ? <VerticalName name={paramFormatter(type)(fx.type)} /> : null
      }
    />
  );
}

/** The plate under each FX strip: its icon and role. */
const ROLE: Readonly<Record<FxStripKind, { icon: string; label: string }>> = {
  chorus: { icon: "fx1", label: "Chorus/Delay" },
  reverb: { icon: "fx2", label: "Reverb" },
};

/** The plate under an FX strip. */
export function FxCategory({ block }: { block: FxStripKind }) {
  return <CategoryPlate icon={stripIcon(ROLE[block].icon)} label={ROLE[block].label} />;
}
