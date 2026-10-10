/** Hard-knee compressor transfer curve: input level (x) vs output level (y). */
interface Props {
  /** Threshold, in the same units as `max`. */
  threshold: number;
  /** Compression ratio above the threshold (Infinity for a limiter). */
  ratio: number;
  /** Full-scale input level. */
  max: number;
}

const W = 88;
const H = 60;

export function TransferCurve({ threshold, ratio, max }: Props) {
  const x = (v: number) => (v / max) * W;
  const y = (v: number) => H - (v / max) * H;
  const out = threshold + (max - threshold) / ratio;
  return (
    <svg width={W} height={H} viewBox={`0 0 ${W} ${H}`} style={{ background: "#112", borderRadius: 3 }}>
      <line x1={0} y1={H} x2={W} y2={0} stroke="#2a2a4a" strokeDasharray="2 2" />
      <line x1={x(threshold)} y1={0} x2={x(threshold)} y2={H} stroke="#2a2a4a" />
      <polyline
        points={`0,${H} ${x(threshold)},${y(threshold)} ${W},${y(out)}`}
        fill="none"
        stroke="#fc8"
        strokeWidth={2}
      />
    </svg>
  );
}
