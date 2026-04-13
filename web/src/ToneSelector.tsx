import { useState, useEffect, useRef, useCallback } from "react";
import { TONE_BANK_GROUPS, type ToneBank } from "./toneBanks";
import type { ToneCatalog, ToneEntry } from "./toneCatalog";
import { CATEGORIES, CATEGORY_IDS, categoryAbbrev } from "./categories";
import css from "./ToneSelector.module.css";

type BrowseMode = "bank" | "category";

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

/** Short bank badge from MSB for category-mode rows. */
function bankBadge(msb: number): string {
  switch (msb) {
    case 89: return "SN-A";
    case 95: return "SN-S";
    case 88: return "SN-D";
    case 87: return "PCM-S";
    case 86: return "PCM-D";
    case 121: return "GM2";
    case 120: return "GM2-D";
    case 97: return "ExPCM";
    case 96: return "ExDRM";
    default: return `${msb}`;
  }
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

  const [mode, setMode] = useState<BrowseMode>("bank");
  const [selectedBank, setSelectedBank] = useState<ToneBank | null>(
    initialBank,
  );
  const [selectedCategory, setSelectedCategory] = useState<number | null>(null);
  const [tones, setTones] = useState<DisplayEntry[]>([]);
  const [loading, setLoading] = useState(false);

  // Subscribe to catalog updates for the selected bank (bank mode).
  useEffect(() => {
    if (mode !== "bank" || !selectedBank) return;

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
  }, [mode, selectedBank, catalog]);

  // Load category tones when category is selected.
  useEffect(() => {
    if (mode !== "category" || selectedCategory === null) {
      if (mode === "category") setTones([]);
      return;
    }

    catalog.ensureAllFactoryLoaded();
    const entries = catalog.getByCategory(selectedCategory);
    setTones(entries.map((e, i) => ({ ...e, globalIndex: i + 1 })));
    setLoading(false);
  }, [mode, selectedCategory, catalog]);

  // Scroll to active tone when list loads
  const toneListRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (tones.length > 0) {
      requestAnimationFrame(() => {
        const el = toneListRef.current?.querySelector("[data-active]");
        el?.scrollIntoView({ block: "center" });
      });
    }
  }, [tones]);

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

  const handleModeSwitch = useCallback((newMode: BrowseMode) => {
    setMode(newMode);
    setTones([]);
    if (newMode === "category") {
      setSelectedCategory(null);
    }
  }, []);

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
          {/* Left panel: bank list or category grid */}
          {mode === "bank" ? (
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
          ) : (
            <div className={css.categoryGrid}>
              {CATEGORY_IDS.map((id) => (
                <button
                  key={id}
                  className={`${css.categoryTile} ${selectedCategory === id ? css.categoryTileActive : ""}`}
                  onClick={() => setSelectedCategory(id)}
                >
                  {CATEGORIES[id]}
                </button>
              ))}
            </div>
          )}

          {/* Right panel: tone list */}
          <div className={css.toneList} ref={toneListRef}>
            {tones.length === 0 && loading ? (
              <div className={css.loading}>Loading tones...</div>
            ) : tones.length === 0 && (selectedBank || selectedCategory !== null) ? (
              <div className={css.loading}>
                {mode === "category" && selectedCategory === null
                  ? "Select a category"
                  : "No tones found"}
              </div>
            ) : tones.length === 0 && mode === "category" ? (
              <div className={css.loading}>Select a category</div>
            ) : (
              <>
                {tones.map((entry) => {
                  const isActive =
                    entry.msb === currentMsb &&
                    entry.lsb === currentLsb &&
                    entry.pc === currentPC;
                  return (
                    <button
                      key={`${entry.msb}:${entry.lsb}:${entry.pc}`}
                      {...(isActive ? { "data-active": true } : {})}
                      className={`${css.toneItem} ${isActive ? css.toneActive : ""}`}
                      onClick={() => handleSelect(entry)}
                    >
                      <span className={css.toneNumber}>
                        {String(entry.globalIndex).padStart(4, "0")}
                      </span>
                      {mode === "category" ? (
                        <span className={css.toneBadge}>
                          {bankBadge(entry.msb)}
                        </span>
                      ) : (
                        <span className={css.toneCategory}>
                          {categoryAbbrev(entry.category)}
                        </span>
                      )}
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

        {/* Mode toggle bar */}
        <div className={css.modeBar}>
          <button
            className={`${css.modeButton} ${mode === "category" ? css.modeActive : ""}`}
            onClick={() => handleModeSwitch("category")}
          >
            By Category
          </button>
          <button
            className={`${css.modeButton} ${mode === "bank" ? css.modeActive : ""}`}
            onClick={() => handleModeSwitch("bank")}
          >
            By Tone &amp; Bank
          </button>
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
