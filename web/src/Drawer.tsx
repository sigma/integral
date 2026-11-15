/**
 * A settings drawer: a panel at the right of the pages, headed by its
 * title, its actions and a close button, its groups stacked below. What it
 * holds is the caller's (the Part settings drawer); it stays up across
 * pages for as long as the caller shows it.
 */

import type { ReactNode } from "react";
import css from "./Drawer.module.css";

/** Props of {@link Drawer}. */
export interface DrawerProps {
  /** What the drawer is for, as a screen reader names it ("Part settings"). */
  label: string;
  /** The header's title: what the drawer edits now ("Part 3 · ch 3 · Grand Piano"). */
  title: ReactNode;
  /** Controls in the header, between the title and the close button. */
  actions?: ReactNode;
  onClose: () => void;
  /** The groups, stacked top to bottom. */
  children: ReactNode;
}

/** The drawer's shell: header, then its groups stacked, scrolling when they are taller than the window. */
export function Drawer({ label, title, actions, onClose, children }: DrawerProps) {
  return (
    <aside className={css.drawer} aria-label={label}>
      <header className={css.header}>
        <span className={css.title}>{title}</span>
        {actions}
        <button className={css.close} onClick={onClose} title={`Close ${label}`} aria-label="Close">
          ✕
        </button>
      </header>
      <div className={css.groups}>{children}</div>
    </aside>
  );
}
