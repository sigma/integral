/**
 * The Part settings drawer's Init items (#288, #591): Init Part… in its
 * header, Sound Ctrl Init… on its OFFSET group. Both are the shared
 * `InitItem`, which confirms, runs and reads back, styled as the drawer's.
 */

import { deviceSpec } from "./deviceSpec";
import { InitItem } from "./InitFlow";
import css from "./PartInitItems.module.css";

/** Init Part…: resets Part `part` (0-based), the one the drawer shows. */
export function InitPartItem({ part }: { part: number }) {
  return (
    <InitItem scope={{ kind: "part", part }} className={css.item}>
      Init Part…
    </InitItem>
  );
}

/** Sound Ctrl Init…: resets the Sound Ctrl offsets of every Part, and says so. */
export function SoundCtrlInitItem() {
  const parts = deviceSpec().part_count;
  return (
    <InitItem scope={{ kind: "sound-ctrl" }} className={`${css.item} ${css.group}`}>
      Sound Ctrl Init (all {parts} Parts)…
    </InitItem>
  );
}
