import {
  SynthFader,
  SectionPanel,
  FaderGroup,
} from "../synth-ui";
import type { PcmDrumCommon, PcmDrumCommon2 } from "./types";

export function CommonPanel({
  common,
  common2: _common2,
  onKitLevelChange,
}: {
  common: PcmDrumCommon;
  common2: PcmDrumCommon2 | null;
  onKitLevelChange: (value: number) => void;
}) {
  return (
    <SectionPanel label="COMMON" accentColor="#8cf">
      <FaderGroup>
        <SynthFader label="Kit Lv" value={common.kitLevel} min={0} max={127} defaultValue={127}
          onChange={onKitLevelChange} formatValue={(v) => String(v)} trackHeight={160}
          title="Kit Level" />
      </FaderGroup>
    </SectionPanel>
  );
}
