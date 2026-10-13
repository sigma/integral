import type { ReactNode } from "react";
import type { ParameterFamily } from "../../pkg/integral_wasm.js";
import { familyVar } from "./family";
import css from "./SectionPanel.module.css";

/**
 * Section colours outside the Parameter family palette, by name. Sections
 * pick one by name, so no colour literal can reach a panel.
 */
const SECTION_COLOURS = {
  /** Per-key drum sections (DRUM KEY, DRUM INST). */
  drum: "#f93",
} as const;

/** A named section colour that is not a Parameter family. */
export type SectionColour = keyof typeof SECTION_COLOURS;

type Accent =
  | {
      /** Parameter family: colours the top border and header underline. */
      family: ParameterFamily;
      sectionColour?: never;
    }
  | {
      family?: never;
      /** A non-family section colour, for sections no family describes. */
      sectionColour: SectionColour;
    };

type Props = Accent & {
  label: string;
  children: ReactNode;
};

export function SectionPanel(props: Props) {
  const { label, children } = props;
  const accent =
    props.family === undefined
      ? SECTION_COLOURS[props.sectionColour]
      : familyVar(props.family);
  return (
    <div className={css.panel} style={{ borderTopColor: accent }}>
      <div className={css.header} style={{ borderBottomColor: accent }}>
        {label}
      </div>
      <div className={css.body}>{children}</div>
    </div>
  );
}
