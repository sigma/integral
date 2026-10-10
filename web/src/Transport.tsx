import { NumberInput } from "./NumberInput";
import { useSequencer, type SequencerEngine } from "./sequencer";
import css from "./Transport.module.css";

const MIN_BPM = 20;
const MAX_BPM = 300;
const MIN_VELOCITY = 1;
const MAX_VELOCITY = 127;

interface Props {
  engine: SequencerEngine;
}

/** Sequencer transport for the tab bar: Play/Stop, Rec with its default velocity, tempo and Panic. */
export function Transport({ engine }: Props) {
  const { playing, bpm, armed, defaultVelocity } = useSequencer(engine);

  return (
    <div className={css.transport}>
      <button
        className={`${css.button} ${playing ? css.playing : ""}`}
        onClick={() => engine.togglePlay()}
        title={playing ? "Stop (Space)" : "Play (Space)"}
      >
        {playing ? "■" : "▶"}
      </button>
      <button
        className={`${css.button} ${armed ? css.armed : ""}`}
        onClick={() => engine.setArmed(!armed)}
        title="Arm Step input"
      >
        ● Rec
      </button>
      <label className={css.field} title="Velocity written by Step input">
        <NumberInput
          className={css.number}
          value={defaultVelocity}
          min={MIN_VELOCITY}
          max={MAX_VELOCITY}
          onCommit={(v) => engine.setDefaultVelocity(v)}
        />
        VEL
      </label>
      <label className={css.field}>
        <NumberInput className={css.number} value={bpm} min={MIN_BPM} max={MAX_BPM} onCommit={(v) => engine.setBpm(v)} />
        BPM
      </label>
      <button className={css.button} onClick={() => engine.panic()} title="All notes off on every channel">
        Panic
      </button>
    </div>
  );
}
