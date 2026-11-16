/**
 * The Part settings drawer ([#280](https://github.com/sigma/integral/issues/280),
 * [PRD #588](https://github.com/sigma/integral/issues/588)): every group of
 * core's Part settings registry for the Part selected in the Mixer, stacked,
 * each a Grid Section of house knobs, lists and switches. It follows the
 * selection, and its edits go to the device through core.
 *
 * SCALE ([#600](https://github.com/sigma/integral/issues/600)) shows the
 * System Scale Tune Switch it depends on, read-only with a way to the System
 * page's Tuning, and dims while it is OFF. Core mirrors what the device does
 * to the other SCALE fields on a write (docs/midi/99-undocumented.md §11):
 * the drawer shows the state it holds.
 */

import type { ReactNode } from "react";
import {
  PartSetting,
  PartSettingGroup,
  Relevance,
  SystemField,
  SystemGroup,
  part_setting_fields,
  part_setting_group,
  part_setting_in_part_block,
  part_setting_label,
  part_scale_ignored_note,
  part_scale_switched_off_note,
  part_setting_name,
  part_setting_relevance,
  part_setting_spec,
  system_field_spec,
  system_group_label,
  tone_ignores_scale,
} from "../pkg/integral_wasm.js";
import { cached } from "./cached";
import { Drawer } from "./Drawer";
import { InitGuard } from "./InitFlow";
import { GridSection, loadedRelevance, specBinding, type Binding } from "./layouts";
import { memo } from "./layouts/fields";
import { partSettingsLayout } from "./layouts/part-settings/sections";
import type { IntegraService } from "./integra";
import { selectedPartChannel } from "./ScopeControls";
import css from "./PartSettingsDrawer.module.css";
import type { UseMixerResult } from "./useMixer";
import { useSystemCommon } from "./useSystemCommon";

/** Core's spec of a Part setting, one wasm object per field. */
const specOf = cached(part_setting_spec);
/** Core's CLI name of a Part setting, the key of `PartState.settings`. */
const nameOf = cached(part_setting_name);
/** Core's label of a Part setting. */
const labelOf = cached(part_setting_label);

/** The registry's groups, in its order: the order the drawer stacks them. */
const registryGroups = memo((): PartSettingGroup[] => [...new Set(part_setting_fields().map(part_setting_group))]);

/** Whether every field of `group` sits in the Part's own block, which a Part Init resets (not VOICE's Common). */
const inPartBlock = cached((group: PartSettingGroup): boolean =>
  part_setting_fields()
    .filter((field) => part_setting_group(field) === group)
    .every(part_setting_in_part_block),
);

/** Core's spec of the System Scale Tune Switch, which SCALE depends on. */
const scaleTuneSwitch = memo(() => system_field_spec(SystemField.ScaleTuneSwitch));

interface ScaleNotesProps {
  /** The System Scale Tune Switch's raw value, none until read. */
  switchValue: number | undefined;
  /** The switch is OFF, so SCALE is switched off. */
  switchedOff: boolean;
  /** The Part's tone ignores the scale. */
  ignored: boolean;
  /** Shows the System page's Tuning; no link without it. */
  onOpenSystem?: () => void;
}

/** What SCALE depends on: the System Scale Tune Switch, read-only, and the tone. */
function ScaleNotes({ switchValue, switchedOff, ignored, onOpenSystem }: ScaleNotesProps) {
  const spec = scaleTuneSwitch();
  return (
    <div className={css.notes}>
      <p className={css.status}>
        System {spec.label}: {switchValue === undefined ? "—" : spec.formatValue(switchValue)}
        {onOpenSystem && (
          <button type="button" className={css.link} onClick={onOpenSystem}>
            System {system_group_label(SystemGroup.Tuning)}…
          </button>
        )}
      </p>
      {switchedOff && (
        <p className={css.note} role="note">
          {part_scale_switched_off_note()}
        </p>
      )}
      {ignored && (
        <p className={css.note} role="note">
          {part_scale_ignored_note()}
        </p>
      )}
    </div>
  );
}

interface Props {
  mixer: UseMixerResult;
  /** The device, for the System Scale Tune Switch SCALE depends on. */
  service: IntegraService;
  onClose: () => void;
  /** Shows the System page, from SCALE's link to its Tuning. */
  onOpenSystem?: () => void;
  /** Controls in the header, before the close button (Init Part…, #591). */
  headerActions?: ReactNode;
  /** Controls drawn with a group, by group (Sound Ctrl Init… on OFFSET, #591). */
  groupActions?: Partial<Record<PartSettingGroup, ReactNode>>;
}

/** The drawer for the Part selected in the Mixer. */
export function PartSettingsDrawer({ mixer, service, onClose, onOpenSystem, headerActions, groupActions }: Props) {
  const { state } = mixer;
  const index = state.selectedPart;
  const part = state.parts[index];
  // Before the mixer is read, the Parts hold defaults: nothing to edit or name.
  const loaded = !state.loading && part !== undefined;
  const switchValue = useSystemCommon(service).read.values?.get(SystemField.ScaleTuneSwitch);
  // Until it is read, the switch dims nothing.
  const scaleRelevance = (field: PartSetting) => part_setting_relevance(field, switchValue !== 0);
  const relevance = (field: PartSetting): Relevance => (loaded ? scaleRelevance(field) : loadedRelevance(false));
  const scaleNotes = (
    <ScaleNotes
      switchValue={switchValue}
      switchedOff={scaleRelevance(PartSetting.ScaleTuneType) === Relevance.SwitchedOff}
      ignored={loaded && tone_ignores_scale(part.toneBankMsb, part.snaInstrument?.variation, part.snaInstrument?.number)}
      onOpenSystem={onOpenSystem}
    />
  );

  const bind = (field: PartSetting): Binding => {
    const spec = specOf(field);
    const name = labelOf(field);
    const value = part?.settings.get(nameOf(field)) ?? spec.defaultValue;
    // A layout's short label ("Fade Lo") may repeat; the registry's names one field.
    return { ...specBinding(name, spec, value, (v) => mixer.setPartSetting(index, field, v)), accessibleName: name };
  };

  const channel = selectedPartChannel(state);
  const title = [`Part ${index + 1}`, channel && `ch ${channel}`, loaded && (part.toneName.trim() || "—")]
    .filter(Boolean)
    .join(" · ");

  return (
    <Drawer
      label="Part settings"
      title={title}
      actions={headerActions}
      hint="Follows the Part selected in the Mixer: press a strip to switch."
      onClose={onClose}
    >
      {registryGroups().map((group) => {
        const layout = partSettingsLayout(group);
        if (!layout) return null;
        let section = <GridSection layout={layout} bind={bind} relevance={relevance} />;
        // A Sound Ctrl Init resets the offsets alone; a Part Init, its Part's block.
        if (group === PartSettingGroup.Offset) section = <InitGuard area={{ kind: "sound-ctrl" }}>{section}</InitGuard>;
        if (inPartBlock(group)) section = <InitGuard area={{ kind: "part", part: index }}>{section}</InitGuard>;
        return (
          <div key={group} data-part-settings-group={PartSettingGroup[group]}>
            {section}
            {group === PartSettingGroup.Scale && scaleNotes}
            {groupActions?.[group]}
          </div>
        );
      })}
    </Drawer>
  );
}
