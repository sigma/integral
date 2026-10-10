import { useState } from "react";
import { Piano } from "./Piano";
import type { Keyboard } from "./useKeyboard";
import css from "./KeyboardDrawer.module.css";

interface Props {
  keyboard: Keyboard;
}

/** Bottom drawer holding the on-screen piano; opens and closes. */
export function KeyboardDrawer({ keyboard }: Props) {
  const [open, setOpen] = useState(false);
  return (
    <div className={css.drawer}>
      <button className={css.handle} onClick={() => setOpen((o) => !o)} aria-expanded={open}>
        Keyboard {open ? "▾" : "▴"}
      </button>
      {open && <Piano keyboard={keyboard} />}
    </div>
  );
}
