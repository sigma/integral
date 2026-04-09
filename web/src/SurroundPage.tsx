import type { UseMixerResult } from "./useMixer";
import css from "./SurroundPage.module.css";

interface Props {
  mixer: UseMixerResult;
}

export function SurroundPage({ mixer }: Props) {
  const { surround } = mixer.state;

  return (
    <div className={css.page}>
      <div className={css.placeholder}>
        Motional Surround — {surround.enabled ? "ON" : "OFF"}
      </div>
    </div>
  );
}
