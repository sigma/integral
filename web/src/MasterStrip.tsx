import { VolumeFader } from "./VolumeFader";
import { EqSection } from "./EqSection";
import type { EqState } from "./types";
import css from "./ChannelStrip.module.css";
import masterCss from "./MasterStrip.module.css";

interface Props {
  value: number;
  onChange: (value: number) => void;
  eq: EqState;
  eqExpanded: boolean;
  onEqToggle: () => void;
  onEqParam: (paramOffset: number, value: number) => void;
}

export function MasterStrip({
  value,
  onChange,
  eq,
  eqExpanded,
  onEqToggle,
  onEqParam,
}: Props) {
  return (
    <div className={`${css.strip} ${masterCss.override}`}>
      <div className={css.partNumber}>Master</div>
      {eqExpanded && (
        <EqSection
          eq={eq}
          onToggleSwitch={onEqToggle}
          onParam={onEqParam}
          paramBase={0}
        />
      )}
      <div className={css.faderArea}>
        <VolumeFader value={value} onChange={onChange} />
      </div>
    </div>
  );
}
