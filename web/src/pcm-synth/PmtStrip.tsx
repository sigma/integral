import {
  SynthSelect,
  SynthSwitch,
} from "../synth-ui";
import { PcmsPmtField, pcms_pmt_spec } from "../../pkg/integral_wasm.js";
import { switchOptions } from "../fieldSpec";
import { StructureDiagram } from "./StructureDiagram";
import css from "../PcmSynthEditor.module.css";
import type { PcmSynthPmt, PmtParamHandler } from "./types";

export function PmtStrip({
  pmt,
  onChange,
}: {
  pmt: PcmSynthPmt;
  onChange: PmtParamHandler;
}) {
  const set =
    (field: PcmsPmtField, key: Parameters<PmtParamHandler>[1]) =>
    (v: number) =>
      onChange(field, key, v);
  return (
    <div className={css.pmtStrip}>
      <div className={css.commonGroup}>
        <span className={css.groupLabel}>PARTIAL MIX 1&2</span>
        <div className={css.groupRow}>
          <SynthSelect label="Structure" value={pmt.structureType12}
            options={switchOptions(pcms_pmt_spec(PcmsPmtField.StructureType12))}
            onChange={set(PcmsPmtField.StructureType12, "structureType12")}
            title="How partials 1 & 2 are wired: 1 independent, 2 both filters in series, 3-4 through the booster, 5-10 through the ring modulator" />
          <StructureDiagram structure={pmt.structureType12} first={1} />
          <SynthSwitch label="Boost" value={pmt.booster12} vertical
            options={switchOptions(pcms_pmt_spec(PcmsPmtField.Booster12))}
            onChange={set(PcmsPmtField.Booster12, "booster12")} led={false} title="Booster depth for partials 1 & 2: distorts the pair; acts with Structure 3 and 4 only" />
        </div>
      </div>

      <div className={css.commonDivider} />

      <div className={css.commonGroup}>
        <span className={css.groupLabel}>PARTIAL MIX 3&4</span>
        <div className={css.groupRow}>
          <SynthSelect label="Structure" value={pmt.structureType34}
            options={switchOptions(pcms_pmt_spec(PcmsPmtField.StructureType34))}
            onChange={set(PcmsPmtField.StructureType34, "structureType34")}
            title="How partials 3 & 4 are wired: 1 independent, 2 both filters in series, 3-4 through the booster, 5-10 through the ring modulator" />
          <StructureDiagram structure={pmt.structureType34} first={3} />
          <SynthSwitch label="Boost" value={pmt.booster34} vertical
            options={switchOptions(pcms_pmt_spec(PcmsPmtField.Booster34))}
            onChange={set(PcmsPmtField.Booster34, "booster34")} led={false} title="Booster depth for partials 3 & 4: distorts the pair; acts with Structure 3 and 4 only" />
        </div>
      </div>

      <div className={css.commonDivider} />

      <SynthSwitch label="Vel Ctrl" value={pmt.pmtVelocityControl} vertical
        options={switchOptions(pcms_pmt_spec(PcmsPmtField.PmtVelocityControl))}
        onChange={set(PcmsPmtField.PmtVelocityControl, "pmtVelocityControl")} led={false}
        title="Velocity control — how key velocity selects partials" />
    </div>
  );
}
