import { NumberInput } from "./NumberInput";
import { useSequencer, type SequencerEngine } from "./sequencer";
import css from "./Transport.module.css";

const MIN_BPM = 20;
const MAX_BPM = 300;

interface Props {
  engine: SequencerEngine;
}

/** Sequencer transport for the tab bar: Play/Stop, tempo and Panic. */
export function Transport({ engine }: Props) {
  const { playing, bpm } = useSequencer(engine);

  return (
    <div className={css.transport}>
      <button
        className={`${css.button} ${playing ? css.playing : ""}`}
        onClick={() => (playing ? engine.stop() : engine.play())}
        title={playing ? "Stop" : "Play"}
      >
        {playing ? "■" : "▶"}
      </button>
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
