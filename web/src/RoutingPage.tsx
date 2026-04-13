import type { UseMixerResult } from "./useMixer";
import {
  CHORUS_TYPE_NAMES,
  REVERB_TYPE_NAMES,
  CHORUS_OUTPUT_NAMES,
  REVERB_OUTPUT_NAMES,
} from "./fxParams";
import css from "./RoutingPage.module.css";

const OUTPUT_ASSIGN_NAMES = [
  "A", "B", "C", "D",
  "1", "2", "3", "4", "5", "6", "7", "8",
];

const COMP_EQ_OUTPUT_NAMES = [
  "PART", "A", "B", "C", "D",
  "1", "2", "3", "4", "5", "6", "7", "8",
];

const TONE_TYPE_LABELS: Record<number, string> = {
  87: "PCM-S",
  89: "SN-A",
  95: "SN-S",
  86: "PCM-D",
  88: "SN-D",
  121: "GM2",
  97: "ExPCM",
};

interface Props {
  mixer: UseMixerResult;
}

export function RoutingPage({ mixer }: Props) {
  const { state } = mixer;
  const surroundActive = state.surround.enabled;

  return (
    <div className={css.page}>
      {surroundActive && (
        <div className={css.banner}>
          Motional Surround is active — output routing is overridden.
        </div>
      )}

      {/* Section 1: Part Routing Grid */}
      <div className={`${css.section} ${surroundActive ? css.dimmed : ""}`}>
        <div className={css.sectionTitle}>Part Routing</div>
        <div className={css.partGrid}>
          {state.parts.map((part, i) => {
            const typeLabel = TONE_TYPE_LABELS[part.toneBankMsb] ?? "---";
            const chorusPct = part.chorusSend / 127;
            const reverbPct = part.reverbSend / 127;

            return (
              <div key={i} className={css.partCol}>
                <span className={css.partNum}>{i + 1}</span>
                <span className={css.toneType}>{typeLabel}</span>
                <span className={css.toneName} title={part.toneName}>
                  {part.toneName || "---"}
                </span>
                <select
                  className={css.partSelect}
                  value={part.outputAssign}
                  onChange={(e) =>
                    mixer.setPartOutputAssign(i, Number(e.target.value))
                  }
                >
                  {OUTPUT_ASSIGN_NAMES.map((name, v) => (
                    <option key={v} value={v}>
                      {name}
                    </option>
                  ))}
                </select>
                <div className={css.sendRow}>
                  <span className={css.sendLabel}>FX1</span>
                  <div className={css.sendBarBg}>
                    <div
                      className={`${css.sendBarFill} ${css.sendBarChorus}`}
                      style={{ width: `${chorusPct * 100}%` }}
                    />
                  </div>
                </div>
                <div className={css.sendRow}>
                  <span className={css.sendLabel}>FX2</span>
                  <div className={css.sendBarBg}>
                    <div
                      className={`${css.sendBarFill} ${css.sendBarReverb}`}
                      style={{ width: `${reverbPct * 100}%` }}
                    />
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      </div>

      {/* Section 2: Effects Routing */}
      <div className={css.section}>
        <div className={css.sectionTitle}>Effects Routing</div>
        <div className={css.fxBar}>
          <div className={css.fxCard}>
            <span className={css.fxLabel}>Chorus</span>
            <span className={css.fxType}>
              {CHORUS_TYPE_NAMES[state.chorus.type] ?? "---"}
            </span>
            <span className={css.fxOut}>
              Out:{" "}
              <span className={css.fxOutValue}>
                {CHORUS_OUTPUT_NAMES[state.chorus.output] ?? "---"}
              </span>
            </span>
          </div>
          <div className={css.fxCard}>
            <span className={css.fxLabel}>Reverb</span>
            <span className={css.fxType}>
              {REVERB_TYPE_NAMES[state.reverb.type] ?? "---"}
            </span>
            <span className={css.fxOut}>
              Out:{" "}
              <span className={css.fxOutValue}>
                {REVERB_OUTPUT_NAMES[state.reverb.output] ?? "---"}
              </span>
            </span>
          </div>
        </div>
      </div>

      {/* Section 3: Drum Comp+EQ Routing */}
      {state.drumCompEq.enabled && (
        <div className={`${css.section} ${surroundActive ? css.dimmed : ""}`}>
          <div className={css.sectionTitle}>
            Drum Comp+EQ (Part {state.drumCompEq.part + 1})
          </div>
          <div className={css.compEqGrid}>
            {state.drumCompEq.outputAssigns.map((assign, unit) => (
              <div key={unit} className={css.compEqUnit}>
                <span className={css.compEqLabel}>C+EQ{unit + 1}</span>
                <select
                  className={css.compEqSelect}
                  value={assign}
                  onChange={(e) =>
                    mixer.setDrumCompEqOutputAssign(
                      unit,
                      Number(e.target.value),
                    )
                  }
                >
                  {COMP_EQ_OUTPUT_NAMES.map((name, v) => (
                    <option key={v} value={v}>
                      {name}
                    </option>
                  ))}
                </select>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
