import css from "./PartSelector.module.css";

interface Props {
  selectedPart: number;
  onSelect: (part: number) => void;
}

export function PartSelector({ selectedPart, onSelect }: Props) {
  return (
    <div className={css.container}>
      {Array.from({ length: 16 }, (_, i) => (
        <button
          key={i}
          className={`${css.tab} ${i === selectedPart ? css.active : ""}`}
          onClick={() => onSelect(i)}
        >
          {i + 1}
        </button>
      ))}
    </div>
  );
}
