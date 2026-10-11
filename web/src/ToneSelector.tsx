import { useState, useEffect, useRef, useCallback, useMemo } from "react";
import {
  findToneBank,
  toneBankGroups,
  type ToneBank,
  type ToneBankGroup,
} from "./toneBanks";
import { is_drum_kit_bank } from "../pkg/integral_wasm.js";
import type { ToneCatalog, ToneEntry } from "./toneCatalog";
import { CATEGORIES, CATEGORY_IDS, categoryAbbrev } from "./categories";
import { useBankEntries } from "./useBankEntries";
import type { UseExpansionsResult } from "./useExpansions";
import type { Expansion } from "./expansionCatalog";
import { LoadedMarker, NotLoadedMarker } from "./NotLoadedMarker";
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
  expansions: UseExpansionsResult;
  onSelect: (msb: number, lsb: number, pc: number) => void;
  /** Load an Expansion that is not loaded (stages it; nothing loads yet). */
  onLoadExpansion: (expansion: Expansion) => void;
  onClose: () => void;
}

/** Short bank badge for category-mode rows. */
function bankBadge(msb: number, lsb: number): string {
  return findToneBank(msb, lsb)?.badge ?? `${msb}`;
}

/** One entry of the bank list: a bank, or an Expansion with its banks. */
type BankListEntry =
  | { kind: "bank"; bank: ToneBank }
  | { kind: "expansion"; expansion: Expansion; banks: ToneBank[] };

/** Bank list entries of a group, one per Expansion for Expansion banks. */
function bankListEntries(group: ToneBankGroup): BankListEntry[] {
  const entries: BankListEntry[] = [];
  for (const bank of group.banks) {
    const expansion = bank.expansion;
    if (expansion === null) {
      entries.push({ kind: "bank", bank });
      continue;
    }
    const last = entries[entries.length - 1];
    if (last?.kind === "expansion" && last.expansion.name === expansion.name) {
      last.banks.push(bank);
    } else {
      entries.push({ kind: "expansion", expansion, banks: [bank] });
    }
  }
  return entries;
}

export function ToneSelector({
  partIndex,
  currentMsb,
  currentLsb,
  currentPC,
  catalog,
  expansions,
  onSelect,
  onLoadExpansion,
  onClose,
}: Props) {
  const initialBank = findToneBank(currentMsb, currentLsb);

  const [mode, setMode] = useState<BrowseMode>("bank");
  const [selectedBank, setSelectedBank] = useState<ToneBank | null>(
    initialBank,
  );
  const [selectedCategory, setSelectedCategory] = useState<number | null>(null);
  const bankTones = useBankEntries(
    catalog,
    mode === "bank" ? selectedBank : null,
  );
  const [categoryTones, setCategoryTones] = useState<ToneEntry[]>([]);

  // Load category tones when category is selected.
  useEffect(() => {
    if (mode !== "category" || selectedCategory === null) {
      setCategoryTones([]);
      return;
    }
    catalog.ensureAllFactoryLoaded();
    setCategoryTones(catalog.getByCategory(selectedCategory));
  }, [mode, selectedCategory, catalog]);

  const loading = mode === "bank" && bankTones.loading;
  const tones: DisplayEntry[] = useMemo(
    () =>
      (mode === "bank" ? bankTones.entries : categoryTones).map((e, i) => ({
        ...e,
        globalIndex: i + 1,
      })),
    [mode, bankTones.entries, categoryTones],
  );

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

  // Opening the picker re-reads which Expansions are loaded.
  const { refresh: refreshExpansions } = expansions;
  useEffect(() => {
    refreshExpansions();
  }, [refreshExpansions]);

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
              {toneBankGroups().map((group) => (
                <div key={group.label}>
                  <div className={css.groupLabel}>{group.label}</div>
                  {bankListEntries(group).map((entry) =>
                    entry.kind === "bank" ? (
                      <BankButton
                        key={bankId(entry.bank)}
                        bank={entry.bank}
                        label={entry.bank.label}
                        selected={selectedBank}
                        onSelect={setSelectedBank}
                      />
                    ) : (
                      <div key={entry.expansion.name} className={css.expansionRow}>
                        <span className={css.expansionName}>
                          {entry.expansion.name}
                          {expansions.slots !== null &&
                            !expansions.isUnloaded(entry.expansion) && (
                              <LoadedMarker />
                            )}
                          {expansions.isUnloaded(entry.expansion) && (
                            <>
                              <NotLoadedMarker expansion={entry.expansion.name} />
                              <button
                                className={css.loadButton}
                                onClick={() => onLoadExpansion(entry.expansion)}
                                title={`Stage ${entry.expansion.name} on the Expansions page`}
                              >
                                Load
                              </button>
                            </>
                          )}
                        </span>
                        {entry.banks.map((bank) => (
                          <BankButton
                            key={bankId(bank)}
                            bank={bank}
                            label={is_drum_kit_bank(bank.msb) ? "Drum" : "Tone"}
                            selected={selectedBank}
                            onSelect={setSelectedBank}
                          />
                        ))}
                      </div>
                    ),
                  )}
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
                  const unloaded = expansions.unloadedExpansionOf(
                    entry.msb,
                    entry.lsb,
                  );
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
                          {bankBadge(entry.msb, entry.lsb)}
                        </span>
                      ) : (
                        <span className={css.toneCategory}>
                          {categoryAbbrev(entry.category)}
                        </span>
                      )}
                      <span className={css.toneName}>{entry.name}</span>
                      {mode === "category" && unloaded !== null && (
                        <NotLoadedMarker expansion={unloaded} />
                      )}
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

function bankId(bank: ToneBank): string {
  return `${bank.msb}-${bank.lsbs.join()}`;
}

function BankButton({
  bank,
  label,
  selected,
  onSelect,
}: {
  bank: ToneBank;
  label: string;
  selected: ToneBank | null;
  onSelect: (bank: ToneBank) => void;
}) {
  const isActive = selected !== null && bankId(selected) === bankId(bank);
  return (
    <button
      className={`${css.bankButton} ${isActive ? css.bankActive : ""}`}
      onClick={() => onSelect(bank)}
    >
      {label}
    </button>
  );
}
