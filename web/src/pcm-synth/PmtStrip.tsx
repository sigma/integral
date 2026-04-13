import {
  SynthSelect,
  SynthSwitch,
} from "../synth-ui";
import css from "../PcmSynthEditor.module.css";
import type { PcmSynthPmt } from "./types";
import { STRUCTURE_TYPE_NAMES, BOOSTER_NAMES, PMT_VEL_CTRL_NAMES } from "./types";

export function PmtStrip({
  pmt,
  onChange,
}: {
  pmt: PcmSynthPmt;
  onChange: (offset: number, value: number) => void;
}) {
  return (
    <div className={css.pmtStrip}>
      <div className={css.commonGroup}>
        <span className={css.groupLabel}>PARTIAL MIX 1&2</span>
        <div className={css.groupRow}>
          <SynthSelect label="Structure" value={pmt.structureType12}
            options={STRUCTURE_TYPE_NAMES.map((l, i) => ({ value: i, label: l }))}
            onChange={(v) => onChange(0x00, v)}
            title="How partials 1 & 2 are combined: 1=Independent, 2=Stacked filters, 3-4=Booster, 5-6=Ring mod, 7-9=Filter+Ring combos, 10=Mix+Boost" />
          <SynthSwitch label="Boost" value={pmt.booster12} vertical
            options={BOOSTER_NAMES.map((l, i) => ({ value: i, label: l }))}
            onChange={(v) => onChange(0x01, v)} led={false} title="Output boost for partials 1 & 2" />
        </div>
      </div>

      <div className={css.commonDivider} />

      <div className={css.commonGroup}>
        <span className={css.groupLabel}>PARTIAL MIX 3&4</span>
        <div className={css.groupRow}>
          <SynthSelect label="Structure" value={pmt.structureType34}
            options={STRUCTURE_TYPE_NAMES.map((l, i) => ({ value: i, label: l }))}
            onChange={(v) => onChange(0x02, v)}
            title="How partials 3 & 4 are combined: 1=Independent, 2=Stacked filters, 3-4=Booster, 5-6=Ring mod, 7-9=Filter+Ring combos, 10=Mix+Boost" />
          <SynthSwitch label="Boost" value={pmt.booster34} vertical
            options={BOOSTER_NAMES.map((l, i) => ({ value: i, label: l }))}
            onChange={(v) => onChange(0x03, v)} led={false} title="Output boost for partials 3 & 4" />
        </div>
      </div>

      <div className={css.commonDivider} />

      <SynthSwitch label="Vel Ctrl" value={pmt.pmtVelocityControl} vertical
        options={PMT_VEL_CTRL_NAMES.map((l, i) => ({ value: i, label: l }))}
        onChange={(v) => onChange(0x04, v)} led={false}
        title="Velocity control — how key velocity selects partials" />
    </div>
  );
}
