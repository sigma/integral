import { SynthFader } from "./SynthFader";
import css from "./ADSREnvelope.module.css";

interface FaderDef {
  label: string;
  value: number;
  min: number;
  max: number;
  defaultValue: number;
  onChange: (v: number) => void;
  formatValue?: (v: number) => string;
}

interface Props {
  /** "AD" = 2 faders, "ADSR" = 4 faders. */
  mode?: "AD" | "ADSR";
  attack: FaderDef;
  decay: FaderDef;
  sustain?: FaderDef;
  release?: FaderDef;
  /** Optional extra fader (e.g. Depth, Env Depth). */
  extra?: FaderDef;
  compact?: boolean;
}

/** Simple reactive envelope shape diagram. */
function EnvelopeDiagram({
  mode,
  aNorm,
  dNorm,
  sNorm,
  rNorm,
}: {
  mode: "AD" | "ADSR";
  aNorm: number;
  dNorm: number;
  sNorm: number;
  rNorm: number;
}) {
  const w = 80;
  const h = 24;
  const pad = 2;

  if (mode === "AD") {
    const aX = pad + aNorm * (w * 0.4);
    const dX = aX + dNorm * (w * 0.4);
    const path = `M ${pad} ${h - pad} L ${aX} ${pad} L ${dX} ${h - pad}`;
    return (
      <svg className={css.diagram} viewBox={`0 0 ${w} ${h}`} width={w} height={h}>
        <path d={path} fill="none" stroke="#fc8" strokeWidth="1.5" />
      </svg>
    );
  }

  // ADSR
  const seg = (w - pad * 2) / 4;
  const aX = pad + aNorm * seg;
  const sY = pad + (1 - sNorm) * (h - pad * 2);
  const dX = aX + dNorm * seg;
  const sustainEnd = dX + seg * 0.8;
  const rX = sustainEnd + rNorm * seg;
  const path = `M ${pad} ${h - pad} L ${aX} ${pad} L ${dX} ${sY} L ${sustainEnd} ${sY} L ${rX} ${h - pad}`;

  return (
    <svg className={css.diagram} viewBox={`0 0 ${w} ${h}`} width={w} height={h}>
      <path d={path} fill="none" stroke="#fc8" strokeWidth="1.5" />
    </svg>
  );
}

function normalize(v: number, min: number, max: number): number {
  return (v - min) / (max - min || 1);
}

export function ADSREnvelope({
  mode = "ADSR",
  attack,
  decay,
  sustain,
  release,
  extra,
  compact,
}: Props) {
  const aNorm = normalize(attack.value, attack.min, attack.max);
  const dNorm = normalize(decay.value, decay.min, decay.max);
  const sNorm = sustain ? normalize(sustain.value, sustain.min, sustain.max) : 0.5;
  const rNorm = release ? normalize(release.value, release.min, release.max) : 0.5;

  return (
    <div className={css.envelope}>
      <EnvelopeDiagram mode={mode} aNorm={aNorm} dNorm={dNorm} sNorm={sNorm} rNorm={rNorm} />
      <div className={css.faders}>
        <SynthFader {...attack} compact={compact} />
        <SynthFader {...decay} compact={compact} />
        {mode === "ADSR" && sustain && <SynthFader {...sustain} compact={compact} />}
        {mode === "ADSR" && release && <SynthFader {...release} compact={compact} />}
        {extra && <SynthFader {...extra} compact={compact} />}
      </div>
    </div>
  );
}

/** Shorthand for AD envelope (2 faders + optional depth). */
export function ADEnvelope(props: Omit<Props, "mode" | "sustain" | "release">) {
  return <ADSREnvelope {...props} mode="AD" />;
}
