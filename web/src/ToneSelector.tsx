import { useState, useEffect, useRef, useCallback } from "react";
import type { IntegraService } from "./integra";
import { TONE_BANK_GROUPS, type ToneBank } from "./toneBanks";
import css from "./ToneSelector.module.css";

interface ToneEntry {
  msb: number;
  lsb: number;
  pc: number;
  globalIndex: number; // 1-based display number across all LSBs
  name: string;
}

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
  const initialBank = findBank(currentMsb, currentLsb);

  const [selectedBank, setSelectedBank] = useState<ToneBank | null>(
    initialBank,
  );
  const [tones, setTones] = useState<ToneEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const cacheRef = useRef<Map<string, ToneEntry[]>>(new Map());

  // Load tone names when bank changes — query all LSBs and merge
  useEffect(() => {
    if (!selectedBank) return;

    const key = `${selectedBank.msb}:${selectedBank.lsbs.join(",")}`;
    const cached = cacheRef.current.get(key);
    if (cached) {
      setTones(cached);
      return;
    }

    setLoading(true);
    setTones([]);

    // Incrementally load pages — each page of 64 entries updates the list
    // as it arrives so the user sees results immediately.
    let cancelled = false;
    const accumulated: ToneEntry[] = [];

    (async () => {
      for (const lsb of selectedBank.lsbs) {
        for (const start of [0, 64]) {
          if (cancelled) return;
          const page = await service.requestToneCatalogPage(
            selectedBank.msb,
            lsb,
            start,
            64,
          );
          if (cancelled) return;
          console.log(`[tone-catalog] page LSB=${lsb} start=${start}: ${page.length} entries, total=${accumulated.length + page.length}`);

          for (const e of page) {
            accumulated.push({
              msb: e.msb,
              lsb: e.lsb,
              pc: e.pc,
              globalIndex: accumulated.length + 1,
              name: e.name,
            });
          }
          setTones([...accumulated]);
        }
      }

      cacheRef.current.set(key, accumulated);
      setLoading(false);
    })();

    return () => {
      cancelled = true;
    };
  }, [selectedBank, service]);

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
    (entry: ToneEntry) => {
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
            {loading ? (
              <div className={css.loading}>Loading tones...</div>
            ) : tones.length === 0 && selectedBank ? (
              <div className={css.loading}>No tones found</div>
            ) : (
              tones.map((entry) => {
                const isActive =
                  entry.msb === currentMsb &&
                  entry.lsb === currentLsb &&
                  entry.pc === currentPC;
                return (
                  <button
                    key={`${entry.lsb}-${entry.pc}`}
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
      if (bank.msb === msb && bank.lsbs.includes(lsb)) return bank;
    }
  }
  return null;
}
