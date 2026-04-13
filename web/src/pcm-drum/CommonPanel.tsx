import {
  SynthFader,
  SectionPanel,
  FaderGroup,
} from "../synth-ui";
import type { PcmDrumCommon, PcmDrumCommon2 } from "./types";

export function CommonPanel({
  common,
  common2: _common2,
  onChange,
}: {
  common: PcmDrumCommon;
  common2: PcmDrumCommon2 | null;
  onChange: (offset: number, value: number) => void;
}) {
  return (
    <SectionPanel label="COMMON" accentColor="#8cf">
      <FaderGroup>
        <SynthFader label="Kit Lv" value={common.kitLevel} min={0} max={127} defaultValue={127}
          onChange={(v) => onChange(0x0C, v)} formatValue={(v) => String(v)} compact
          title="Kit Level" />
      </FaderGroup>
    </SectionPanel>
  );
}
