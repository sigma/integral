import { useEffect, useRef, useState, type CSSProperties, type PointerEvent } from "react";
import { WasmSequencer } from "../pkg/integral_wasm.js";
import { NumberInput } from "./NumberInput";
import { Piano } from "./Piano";
import { useSequencer, type SequencerEngine, type TrackView } from "./sequencer";
import { noteName, type Keyboard } from "./useKeyboard";
import type { UseMixerResult } from "./useMixer";
import css from "./SequencerPage.module.css";

const MAX_VELOCITY = 127;

interface Props {
  engine: SequencerEngine;
  mixer: UseMixerResult;
  keyboard: Keyboard;
}

/** The global Step under the playhead, polled once per frame while playing. */
function usePlayStep(engine: SequencerEngine, playing: boolean): number | undefined {
  const [step, setStep] = useState<number>();
  useEffect(() => {
    if (!playing) {
      setStep(undefined);
      return;
    }
    let frame = 0;
    const poll = () => {
      setStep(engine.playStep());
      frame = requestAnimationFrame(poll);
    };
    poll();
    return () => cancelAnimationFrame(frame);
  }, [engine, playing]);
  return step;
}

/** Chords that start inside the Track, with their displayed span. */
function chords(track: TrackView) {
  return track.steps
    .filter((s) => s.step < track.length)
    .map(({ step, notes }) => ({
      step,
      span: Math.min(Math.max(...notes.map((n) => n.length)), track.length - step),
      velocity: Math.max(...notes.map((n) => n.velocity)),
      label: notes.map((n) => noteName(n.pitch)).join(" "),
    }));
}

/** Sequencer tab: the Track × Step grid, the selected Track's controls and the piano. */
export function SequencerPage({ engine, mixer, keyboard }: Props) {
  const { playing, cursor, tracks } = useSequencer(engine);
  const playStep = usePlayStep(engine, playing);
  const selected = mixer.state.selectedPart;
  const selectedTrack = tracks[selected];
  const gridVars = { "--steps": WasmSequencer.maxSteps() } as CSSProperties;

  return (
    <div className={css.page}>
      <div className={css.scroller} style={gridVars}>
        {tracks.map((track, i) => (
          <div key={i} className={css.row}>
            <button
              className={`${css.label} ${i === selected ? css.labelSelected : ""}`}
              onClick={() => mixer.selectPart(i)}
            >
              <span className={css.partNumber}>{i + 1}</span>
              {mixer.state.parts[i]?.toneName}
            </button>
            <div className={css.steps}>
              {Array.from({ length: WasmSequencer.maxSteps() }, (_, step) => {
                const classes = [css.cell];
                if (step >= track.length) classes.push(css.beyond);
                else if (playStep !== undefined && step === playStep % track.length) classes.push(css.playhead);
                if (i === selected && step === cursor) classes.push(css.cursor);
                return (
                  <div
                    key={step}
                    className={classes.join(" ")}
                    onClick={() => {
                      mixer.selectPart(i);
                      engine.setCursor(step);
                    }}
                  />
                );
              })}
              {chords(track).map((c) => (
                <div
                  key={c.step}
                  className={css.block}
                  style={{ "--start": c.step, "--span": c.span, "--vel": c.velocity / MAX_VELOCITY } as CSSProperties}
                >
                  {c.label}
                </div>
              ))}
            </div>
          </div>
        ))}
        {selectedTrack && (
          <VelocityLane engine={engine} track={selectedTrack} trackIndex={selected} />
        )}
      </div>
      <Piano keyboard={keyboard} />
    </div>
  );
}

interface LaneProps {
  engine: SequencerEngine;
  track: TrackView;
  trackIndex: number;
}

/** Len control and per-chord velocity bars for the selected Track. Drag a bar to set its velocity. */
function VelocityLane({ engine, track, trackIndex }: LaneProps) {
  const dragStep = useRef<number | null>(null);
  const steps = WasmSequencer.maxSteps();
  const laneChords = chords(track);

  const apply = (e: PointerEvent<HTMLDivElement>, step: number) => {
    const rect = e.currentTarget.getBoundingClientRect();
    const fraction = 1 - (e.clientY - rect.top) / rect.height;
    const velocity = Math.min(MAX_VELOCITY, Math.max(1, Math.round(MAX_VELOCITY * fraction)));
    engine.edit((seq) => seq.setChordVelocity(trackIndex, step, velocity));
  };

  return (
    <div className={`${css.row} ${css.lane}`}>
      <label className={css.lenControl}>
        Len
        <NumberInput
          className={css.number}
          value={track.length}
          min={1}
          max={steps}
          onCommit={(length) => engine.edit((seq) => seq.setTrackLength(trackIndex, length))}
        />
      </label>
      <div
        className={css.steps}
        onPointerDown={(e) => {
          const rect = e.currentTarget.getBoundingClientRect();
          const step = Math.floor(((e.clientX - rect.left) / rect.width) * steps);
          if (!laneChords.some((c) => c.step === step)) return;
          e.currentTarget.setPointerCapture(e.pointerId);
          dragStep.current = step;
          apply(e, step);
        }}
        onPointerMove={(e) => dragStep.current !== null && apply(e, dragStep.current)}
        onPointerUp={() => (dragStep.current = null)}
      >
        {laneChords.map((c) => (
          <div
            key={c.step}
            className={css.bar}
            style={{ "--start": c.step, "--vel": c.velocity / MAX_VELOCITY } as CSSProperties}
          />
        ))}
      </div>
    </div>
  );
}
