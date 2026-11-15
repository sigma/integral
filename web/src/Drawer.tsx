/**
 * A settings drawer: a panel at the right of the pages, as tall as they are,
 * headed by its title, its actions and ✕, then an optional hint line on
 * what it follows, then its groups stacked below, scrolling when they are
 * taller than the window. The pattern of the Studio Set and Part settings
 * drawers ([#280](https://github.com/sigma/integral/issues/280)): it holds
 * no state and shows whatever its owner gives it, for as long as the owner
 * shows it, across pages. ✕ or Escape closes it.
 */

import { useEffect, type ReactNode } from "react";
import css from "./Drawer.module.css";

/** Props of {@link Drawer}. */
export interface DrawerProps {
  /** What the drawer is for, as a screen reader names it ("Part settings"). */
  label: string;
  /** The header's title: what the drawer edits now ("Part 3 · ch 3 · Grand Piano"); `label` by default. */
  title?: ReactNode;
  /** Controls in the header, between the title and ✕ (an Init…). */
  actions?: ReactNode;
  /** A line under the header, on what the drawer follows. */
  hint?: ReactNode;
  /** The drawer's own sizing, if its groups need one. */
  className?: string;
  onClose: () => void;
  /** The groups ({@link DrawerGroup} or the caller's), stacked top to bottom. */
  children: ReactNode;
}

/** The drawer's shell, closed by ✕ or Escape. */
export function Drawer({ label, title = label, actions, hint, className, onClose, children }: DrawerProps) {
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      // An open list (a picker's popover) takes Escape first: it closes, not the drawer.
      if (e.key === "Escape" && !document.querySelector('[role="listbox"]')) onClose();
    };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [onClose]);
  return (
    <aside className={`${css.drawer} ${className ?? ""}`} aria-label={label}>
      <header className={css.header}>
        <h2 className={css.title}>{title}</h2>
        {actions}
        <button className={css.close} onClick={onClose} title={`Close ${label}`} aria-label={`Close ${label}`}>
          ✕
        </button>
      </header>
      <div className={css.groups}>
        {hint && <p className={css.hint}>{hint}</p>}
        {children}
      </div>
    </aside>
  );
}

interface DrawerGroupProps {
  /** The group's heading (CONTROL, TEMPO). */
  title: string;
  /** Controls beside the heading (a link, an Init…). */
  actions?: ReactNode;
  children: ReactNode;
}

/** One group of a {@link Drawer}: its heading, then its fields. */
export function DrawerGroup({ title, actions, children }: DrawerGroupProps) {
  return (
    <section className={css.group} aria-label={title}>
      <h3 className={css.groupTitle}>
        {title}
        {actions}
      </h3>
      {children}
    </section>
  );
}
