import { routingOf, type Block } from "./structureRouting";
import css from "../PcmSynthEditor.module.css";

const NAMES = { wave: "WG", filter: "TVF", amp: "TVA" } as const;
const JOIN_LABELS = { mix: "+", booster: "B", ring: "R" } as const;

const COL = 30; // px per block column
const BOX_W = 24;
const BOX_H = 12;
const ROW_Y = { upper: 10, lower: 46 } as const;
const MID_Y = 28;
const HEIGHT = 56;

/**
 * The signal path of a PCM Synth Structure Type, as in the Parameter Guide:
 * each partial's WG → TVF → TVA, and where the pair (`first` and `first + 1`)
 * joins through a mixer, the booster (B) or the ring modulator (R).
 */
export function StructureDiagram({ structure, first }: { structure: number; first: number }) {
  const routing = routingOf(structure);
  const label = (b: Block) =>
    b.kind === "booster" ? "B" : `${NAMES[b.kind]}${b.side === "upper" ? first : first + 1}`;

  const boxes: { x: number; y: number; text: string; dark?: boolean }[] = [];
  const lines: string[] = [];
  const lane = (blocks: Block[], y: number) => {
    blocks.forEach((b, i) => {
      boxes.push({ x: i * COL, y, text: label(b) });
      if (i) lines.push(`M ${(i - 1) * COL + BOX_W} ${y} H ${i * COL}`);
    });
    return (blocks.length - 1) * COL + BOX_W; // x where the lane ends
  };
  const upperEnd = lane(routing.upper, ROW_Y.upper);
  const lowerEnd = lane(routing.lower, ROW_Y.lower);
  const join = routing.join;
  let columns = Math.max(routing.upper.length, routing.lower.length);

  if (join) {
    const jx = columns * COL;
    boxes.push({ x: jx + (BOX_W - BOX_H) / 2, y: MID_Y, text: JOIN_LABELS[join.kind], dark: join.kind !== "mix" });
    const jl = jx + (BOX_W - BOX_H) / 2;
    lines.push(`M ${upperEnd} ${ROW_Y.upper} C ${jl} ${ROW_Y.upper} ${jl - 6} ${MID_Y} ${jl} ${MID_Y}`);
    lines.push(`M ${lowerEnd} ${ROW_Y.lower} C ${jl} ${ROW_Y.lower} ${jl - 6} ${MID_Y} ${jl} ${MID_Y}`);
    let prevRight = jl + BOX_H;
    join.after.forEach((b, i) => {
      // Single-letter blocks (B) are narrow, centred in their column.
      const narrow = b.kind === "booster";
      const x = (columns + 1 + i) * COL + (narrow ? (BOX_W - BOX_H) / 2 : 0);
      boxes.push({ x, y: MID_Y, text: label(b), dark: narrow });
      lines.push(`M ${prevRight} ${MID_Y} H ${x}`);
      prevRight = x + (narrow ? BOX_H : BOX_W);
    });
    if (join.lowerDry) {
      // The lower path also bypasses the join, rejoining just after it.
      const back = (columns + 1) * COL - 3;
      lines.push(`M ${lowerEnd} ${ROW_Y.lower} H ${back - 6} C ${back} ${ROW_Y.lower} ${back} ${MID_Y} ${back} ${MID_Y}`);
    }
    columns += 1 + join.after.length;
  }

  const width = (columns - 1) * COL + BOX_W + 2;
  return (
    <svg className={css.structureDiagram} width={width} height={HEIGHT} viewBox={`-1 0 ${width} ${HEIGHT}`}
      role="img" aria-label={`Structure ${structure + 1} signal path`}>
      <g fill="none" stroke="#777" strokeWidth="1">
        {lines.map((d, i) => <path key={i} d={d} />)}
      </g>
      {boxes.map((b, i) => {
        const w = b.text.length <= 1 ? BOX_H : BOX_W;
        return (
          <g key={i}>
            <rect x={b.x} y={b.y - BOX_H / 2} width={w} height={BOX_H} rx="3"
              fill={b.dark ? "#555" : "#1e1e34"} stroke="#888" />
            <text x={b.x + w / 2} y={b.y + 3} textAnchor="middle" fontSize="8" fill="#ccc">{b.text}</text>
          </g>
        );
      })}
    </svg>
  );
}
