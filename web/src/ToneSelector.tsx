import { useState, useEffect, useRef, useCallback } from "react";
import type { IntegraService } from "./integra";
import { TONE_BANK_GROUPS, type ToneBank } from "./toneBanks";
import css from "./ToneSelector.module.css";

interface Props {
  partIndex: number;
  currentMsb: number;
  currentLsb: number;
  currentPC: number;
  service: IntegraService;
  onSelect: (msb: number, lsb: number, pc: number) => void;
  onClose: () => void;
}

export function ToneSelector({
  partIndex,
  currentMsb,
  currentLsb,
  currentPC,
  service,
  onSelect,
  onClose,
}: Props) {
  // Find the initial bank matching the current tone
  const initialBank = findBank(currentMsb, currentLsb);

  const [selectedBank, setSelectedBank] = useState<ToneBank | null>(
    initialBank,
  );
  const [toneNames, setToneNames] = useState<Map<number, string>>(new Map());
  const [loading, setLoading] = useState(false);
  const cacheRef = useRef<Map<string, Map<number, string>>>(new Map());

  // Load tone names when bank changes
  useEffect(() => {
    if (!selectedBank) return;

    const key = `${selectedBank.msb}:${selectedBank.lsb}`;
    const cached = cacheRef.current.get(key);
    if (cached) {
      setToneNames(cached);
      return;
    }

    setLoading(true);
    setToneNames(new Map());

    service.requestToneCatalog(selectedBank.msb, selectedBank.lsb).then(
      (names) => {
        cacheRef.current.set(key, names);
        setToneNames(names);
        setLoading(false);
      },
    );
  }, [selectedBank, service]);

  // Scroll to active tone when list loads
  const toneListRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (
      toneNames.size > 0 &&
      selectedBank?.msb === currentMsb &&
      selectedBank?.lsb === currentLsb
    ) {
      requestAnimationFrame(() => {
        const el = toneListRef.current?.querySelector(
          `[data-pc="${currentPC}"]`,
        );
        el?.scrollIntoView({ block: "center" });
      });
    }
  }, [toneNames, selectedBank, currentMsb, currentLsb, currentPC]);

  const handleSelect = useCallback(
    (pc: number) => {
      if (!selectedBank) return;
      onSelect(selectedBank.msb, selectedBank.lsb, pc);
    },
    [selectedBank, onSelect],
  );

  // Close on Escape
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [onClose]);

  return (
    <div className={css.overlay} onClick={onClose}>
      <div className={css.modal} onClick={(e) => e.stopPropagation()}>
        <div className={css.header}>
          <span className={css.title}>
            Tone Select — Part {partIndex + 1}
          </span>
          <button className={css.closeButton} onClick={onClose}>
            ✕
          </button>
        </div>
        <div className={css.body}>
          <div className={css.bankList}>
            {TONE_BANK_GROUPS.map((group) => (
              <div key={group.label}>
                <div className={css.groupLabel}>{group.label}</div>
                {group.banks.map((bank) => {
                  const isActive =
                    selectedBank?.msb === bank.msb &&
                    selectedBank?.lsb === bank.lsb;
                  return (
                    <button
                      key={`${bank.msb}-${bank.lsb}`}
                      className={`${css.bankButton} ${isActive ? css.bankActive : ""}`}
                      onClick={() => setSelectedBank(bank)}
                    >
                      {bank.label}
                    </button>
                  );
                })}
              </div>
            ))}
          </div>
          <div className={css.toneList} ref={toneListRef}>
            {loading ? (
              <div className={css.loading}>Loading tones...</div>
            ) : toneNames.size === 0 && selectedBank ? (
              <div className={css.loading}>No tones found</div>
            ) : (
              Array.from(toneNames.entries())
                .sort(([a], [b]) => a - b)
                .map(([pc, name]) => {
                  const isActive =
                    selectedBank?.msb === currentMsb &&
                    selectedBank?.lsb === currentLsb &&
                    pc === currentPC;
                  return (
                    <button
                      key={pc}
                      data-pc={pc}
                      className={`${css.toneItem} ${isActive ? css.toneActive : ""}`}
                      onClick={() => handleSelect(pc)}
                    >
                      <span className={css.toneNumber}>
                        {String(pc + 1).padStart(4, "0")}
                      </span>
                      <span className={css.toneName}>{name}</span>
                    </button>
                  );
                })
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

function findBank(msb: number, lsb: number): ToneBank | null {
  for (const group of TONE_BANK_GROUPS) {
    for (const bank of group.banks) {
      if (bank.msb === msb && bank.lsb === lsb) return bank;
    }
  }
  return null;
}
