import { useRef, useState, useLayoutEffect } from "react";
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
  mode?: "AD" | "ADSR";
  attack: FaderDef;
  decay: FaderDef;
  sustain?: FaderDef;
  release?: FaderDef;
  extra?: FaderDef;
  compact?: boolean;
}

function normalize(v: number, min: number, max: number): number {
  return (v - min) / (max - min || 1);
}

/**
 * The envelope is just a row of SynthFaders (which handle their own labels).
 * The curve is drawn as an SVG overlay positioned above the fader group,
 * using a ref to measure the actual width of the envelope faders.
 */
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

  const envRef = useRef<HTMLDivElement>(null);
  const [envWidth, setEnvWidth] = useState(0);

  useLayoutEffect(() => {
    if (!envRef.current) return;
    const ro = new ResizeObserver(() => {
      setEnvWidth(envRef.current?.offsetWidth ?? 0);
    });
    ro.observe(envRef.current);
    setEnvWidth(envRef.current.offsetWidth);
    return () => ro.disconnect();
  }, []);

  // Build the curve path in pixel space
  const curveH = 28;
  const pad = 4;
  let curvePath = "";
  if (envWidth > 0) {
    const w = envWidth;
    if (mode === "AD") {
      const usable = w - pad * 2;
      const aX = pad + aNorm * usable * 0.5;
      const dX = aX + dNorm * usable * 0.4;
      curvePath = `M ${pad} ${curveH - pad} L ${aX} ${pad} L ${dX} ${curveH - pad}`;
    } else {
      const usable = w - pad * 2;
      const seg = usable / 4;
      const aX = pad + aNorm * seg;
      const sY = pad + (1 - sNorm) * (curveH - pad * 2);
      const dX = aX + dNorm * seg;
      const sustainEnd = dX + seg * 0.8;
      const rX = sustainEnd + rNorm * seg;
      curvePath = `M ${pad} ${curveH - pad} L ${aX} ${pad} L ${dX} ${sY} L ${sustainEnd} ${sY} L ${rX} ${curveH - pad}`;
    }
  }

  return (
    <div className={css.envelope}>
      {/* Curve drawn in pixel coords, positioned above the env faders */}
      <div className={css.curveRow} style={{ width: envWidth || "auto", height: curveH }}>
        {envWidth > 0 && (
          <svg width={envWidth} height={curveH} className={css.curve}>
            <path d={curvePath} fill="none" stroke="#fc8" strokeWidth="2" />
          </svg>
        )}
      </div>
      {/* Fader row */}
      <div className={css.faders}>
        {/* Envelope faders — measured by ref */}
        <div className={css.envFaders} ref={envRef}>
          <SynthFader {...attack} compact={compact} />
          <SynthFader {...decay} compact={compact} />
          {mode === "ADSR" && sustain && <SynthFader {...sustain} compact={compact} />}
          {mode === "ADSR" && release && <SynthFader {...release} compact={compact} />}
        </div>
        {/* Extra fader (Depth) separated */}
        {extra && (
          <>
            <div className={css.groupSep} />
            <SynthFader {...extra} compact={compact} />
          </>
        )}
      </div>
    </div>
  );
}

export function ADEnvelope(props: Omit<Props, "mode" | "sustain" | "release">) {
  return <ADSREnvelope {...props} mode="AD" />;
}
