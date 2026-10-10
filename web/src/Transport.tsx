import { useState } from "react";
import { useTransport, type SequencerEngine } from "./sequencer";
import css from "./Transport.module.css";

const MIN_BPM = 20;
const MAX_BPM = 300;

interface Props {
  engine: SequencerEngine;
}

/** Sequencer transport for the tab bar: Play/Stop, tempo and Panic. */
export function Transport({ engine }: Props) {
  const { playing, bpm } = useTransport(engine);
  // Local text so typing "1" on the way to "120" doesn't clamp mid-edit.
  const [draft, setDraft] = useState<string | null>(null);

  const commit = () => {
    if (draft === null) return;
    const value = Number.parseInt(draft, 10);
    if (Number.isFinite(value)) {
      engine.setBpm(Math.min(MAX_BPM, Math.max(MIN_BPM, value)));
    }
    setDraft(null);
  };

  return (
    <div className={css.transport}>
      <button
        className={`${css.button} ${playing ? css.playing : ""}`}
        onClick={() => (playing ? engine.stop() : engine.play())}
        title={playing ? "Stop" : "Play"}
      >
        {playing ? "■" : "▶"}
      </button>
      <label className={css.bpm}>
        <input
          className={css.bpmInput}
          type="number"
          min={MIN_BPM}
          max={MAX_BPM}
          value={draft ?? bpm}
          onChange={(e) => setDraft(e.target.value)}
          onBlur={commit}
          onKeyDown={(e) => e.key === "Enter" && commit()}
        />
        BPM
      </label>
      <button className={css.button} onClick={() => engine.panic()} title="All notes off on every channel">
        Panic
      </button>
    </div>
  );
}
