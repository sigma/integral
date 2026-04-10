import css from "./SynthSwitch.module.css";

interface Props {
  value: number;
  options: { value: number; label: string }[];
  onChange: (value: number) => void;
  label?: string;
  /** Show a colored LED dot next to the active option. */
  led?: boolean;
  /** LED color when lit. Default: "#4dff4d". */
  ledColor?: string;
  /** Render as horizontal toggle (2 states) or vertical list. */
  vertical?: boolean;
}

export function SynthSwitch({
  value,
  options,
  onChange,
  label,
  led = true,
  ledColor = "#4dff4d",
  vertical = false,
}: Props) {
  if (options.length === 2 && !vertical) {
    // Two-state toggle button
    const isOn = value === options[1]!.value;
    return (
      <div className={css.toggle}>
        {label && <span className={css.label}>{label}</span>}
        <button
          className={`${css.toggleBtn} ${isOn ? css.toggleOn : ""}`}
          onClick={() => onChange(isOn ? options[0]!.value : options[1]!.value)}
        >
          {led && <span className={css.led} style={isOn ? { background: ledColor } : undefined} />}
          <span>{isOn ? options[1]!.label : options[0]!.label}</span>
        </button>
      </div>
    );
  }

  // Multi-state vertical list
  return (
    <div className={css.multiSwitch}>
      {label && <span className={css.label}>{label}</span>}
      <div className={css.optionList}>
        {options.map((opt) => {
          const isActive = opt.value === value;
          return (
            <button
              key={opt.value}
              className={`${css.option} ${isActive ? css.optionActive : ""}`}
              onClick={() => onChange(opt.value)}
            >
              {led && (
                <span
                  className={css.led}
                  style={isActive ? { background: ledColor } : undefined}
                />
              )}
              <span>{opt.label}</span>
            </button>
          );
        })}
      </div>
    </div>
  );
}
