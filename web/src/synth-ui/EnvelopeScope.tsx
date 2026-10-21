import { envelopePath, type EnvelopeShape } from "./ADSREnvelope";
import { ScopeScreen } from "./ScopeScreen";

/** An A/D, A/R or A/D/S/R envelope's shape on a scope screen filling its box; no Controls. */
export function EnvelopeScope({ stages, className }: { stages: EnvelopeShape; className?: string }) {
  return (
    <ScopeScreen className={className}>
      <path d={envelopePath(stages)} fill="none" stroke="#fc8" strokeWidth="2" vectorEffect="non-scaling-stroke" />
    </ScopeScreen>
  );
}
