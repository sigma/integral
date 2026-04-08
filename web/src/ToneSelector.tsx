import { useState, useEffect, useRef, useCallback } from "react";
import { TONE_BANK_GROUPS, type ToneBank } from "./toneBanks";
import type { ToneCatalog, ToneEntry } from "./toneCatalog";
import css from "./ToneSelector.module.css";

interface DisplayEntry extends ToneEntry {
  globalIndex: number; // 1-based display number
}

interface Props {
  partIndex: number;
  currentMsb: number;
  currentLsb: number;
  currentPC: number;
  catalog: ToneCatalog;
  onSelect: (msb: number, lsb: number, pc: number) => void;
  onClose: () => void;
}

export function ToneSelector({
  partIndex,
  currentMsb,
  currentLsb,
  currentPC,
  catalog,
  onSelect,
  onClose,
}: Props) {
  const initialBank = findBank(currentMsb, currentLsb);

  const [selectedBank, setSelectedBank] = useState<ToneBank | null>(
    initialBank,
  );
  const [tones, setTones] = useState<DisplayEntry[]>([]);
  const [loading, setLoading] = useState(false);

  // Subscribe to catalog updates for the selected bank.
  useEffect(() => {
    if (!selectedBank) return;

    const updateFromCatalog = (entries: ToneEntry[]) => {
      setTones(entries.map((e, i) => ({ ...e, globalIndex: i + 1 })));
    };

    // Initialize from cache.
    const cached = catalog.get(selectedBank);
    if (cached.length > 0) {
      updateFromCatalog(cached);
    }

    setLoading(!catalog.isComplete(selectedBank));

    // Subscribe to future updates.
    const unsub = catalog.subscribe(selectedBank, (entries) => {
      updateFromCatalog(entries);
      setLoading(!catalog.isComplete(selectedBank));
    });

    // Trigger fetch (no-op if already complete or in progress).
    catalog.fetch(selectedBank);

    return unsub;
  }, [selectedBank, catalog]);

  // Scroll to active tone when list loads
  const toneListRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (tones.length > 0 && selectedBank) {
      requestAnimationFrame(() => {
        const el = toneListRef.current?.querySelector("[data-active]");
        el?.scrollIntoView({ block: "center" });
      });
    }
  }, [tones, selectedBank]);

  const handleSelect = useCallback(
    (entry: DisplayEntry) => {
      onSelect(entry.msb, entry.lsb, entry.pc);
    },
    [onSelect],
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
                    selectedBank?.lsbs.join() === bank.lsbs.join();
                  return (
                    <button
                      key={`${bank.msb}-${bank.lsbs.join()}`}
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
            {tones.length === 0 && loading ? (
              <div className={css.loading}>Loading tones...</div>
            ) : tones.length === 0 && selectedBank ? (
              <div className={css.loading}>No tones found</div>
            ) : (
              <>
                {tones.map((entry) => {
                  const isActive =
                    entry.msb === currentMsb &&
                    entry.lsb === currentLsb &&
                    entry.pc === currentPC;
                  return (
                    <button
                      key={entry.globalIndex}
                      {...(isActive ? { "data-active": true } : {})}
                      className={`${css.toneItem} ${isActive ? css.toneActive : ""}`}
                      onClick={() => handleSelect(entry)}
                    >
                      <span className={css.toneNumber}>
                        {String(entry.globalIndex).padStart(4, "0")}
                      </span>
                      <span className={css.toneName}>{entry.name}</span>
                    </button>
                  );
                })}
                {loading && (
                  <div className={css.loading}>Loading more...</div>
                )}
              </>
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
      if (bank.msb === msb && bank.lsbs.includes(lsb)) return bank;
    }
  }
  return null;
}
