//! Tone selector modal view for browsing and selecting tones by bank.
//!
//! Mirrors the bank-centric mode of the `ToneSelector` React component
//! from `web/src/ToneSelector.tsx`. Category mode is stubbed for now.

use integral_core::factory_catalog::factory_tones;
use nih_plug_vizia::vizia::prelude::*;

// ---------------------------------------------------------------------------
// Bank definitions (mirrors web/src/toneBanks.ts)
// ---------------------------------------------------------------------------

/// A single tone bank identified by MSB and one or more LSBs.
struct ToneBank {
    label: &'static str,
    msb: u8,
    lsbs: &'static [u8],
}

/// A group of related tone banks.
struct ToneBankGroup {
    label: &'static str,
    banks: &'static [ToneBank],
}

/// All tone bank groups, matching the web frontend definitions.
///
/// TODO: migrate to `DeviceSpec` once bank group metadata (MSB/LSB arrays) is
/// added there. For now these remain inline because the data structure is
/// more complex than simple string slices.
static TONE_BANK_GROUPS: &[ToneBankGroup] = &[
    ToneBankGroup {
        label: "SN Acoustic",
        banks: &[
            ToneBank {
                label: "Preset",
                msb: 89,
                lsbs: &[64, 65],
            },
            ToneBank {
                label: "User",
                msb: 89,
                lsbs: &[0, 1],
            },
        ],
    },
    ToneBankGroup {
        label: "SN Synth",
        banks: &[
            ToneBank {
                label: "Preset",
                msb: 95,
                lsbs: &[64, 65, 66, 67, 68, 69, 70, 71, 72],
            },
            ToneBank {
                label: "User",
                msb: 95,
                lsbs: &[0, 1, 2, 3],
            },
        ],
    },
    ToneBankGroup {
        label: "SN Drum",
        banks: &[
            ToneBank {
                label: "Preset",
                msb: 88,
                lsbs: &[64],
            },
            ToneBank {
                label: "User",
                msb: 88,
                lsbs: &[0],
            },
        ],
    },
    ToneBankGroup {
        label: "PCM Synth",
        banks: &[
            ToneBank {
                label: "Preset",
                msb: 87,
                lsbs: &[64, 65, 66, 67, 68, 69, 70],
            },
            ToneBank {
                label: "User",
                msb: 87,
                lsbs: &[0, 1],
            },
        ],
    },
    ToneBankGroup {
        label: "PCM Drum",
        banks: &[
            ToneBank {
                label: "Preset",
                msb: 86,
                lsbs: &[64],
            },
            ToneBank {
                label: "User",
                msb: 86,
                lsbs: &[0],
            },
        ],
    },
    ToneBankGroup {
        label: "GM2",
        banks: &[
            ToneBank {
                label: "Tone",
                msb: 121,
                lsbs: &[0, 1],
            },
            ToneBank {
                label: "Drum",
                msb: 120,
                lsbs: &[0],
            },
        ],
    },
    ToneBankGroup {
        label: "Expansion",
        banks: &[
            ToneBank {
                label: "ExSN1",
                msb: 89,
                lsbs: &[96],
            },
            ToneBank {
                label: "ExSN2",
                msb: 89,
                lsbs: &[97],
            },
            ToneBank {
                label: "ExSN3",
                msb: 89,
                lsbs: &[98],
            },
            ToneBank {
                label: "ExSN4",
                msb: 89,
                lsbs: &[99],
            },
            ToneBank {
                label: "ExSN5",
                msb: 89,
                lsbs: &[100],
            },
            ToneBank {
                label: "ExSN6 Drum",
                msb: 88,
                lsbs: &[101],
            },
            ToneBank {
                label: "ExPCM Tone",
                msb: 97,
                lsbs: &[0, 1, 2, 3],
            },
            ToneBank {
                label: "ExPCM Drum",
                msb: 96,
                lsbs: &[0],
            },
        ],
    },
];

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/// Events emitted by the [`ToneSelector`] view.
pub enum ToneSelectorEvent {
    /// User selected a tone.
    SelectTone { msb: u8, lsb: u8, pc: u8 },
    /// User closed the modal.
    Close,
}

/// Internal events for the tone selector model.
enum ToneSelectorInternal {
    /// User selected a bank (group_index, bank_index).
    SelectBank(usize, usize),
}

// ---------------------------------------------------------------------------
// Model
// ---------------------------------------------------------------------------

/// Display entry for a single tone in the list.
#[derive(Debug, Clone, Data, Lens)]
struct ToneDisplayEntry {
    /// 1-based display number within the bank.
    number: usize,
    /// Bank Select MSB.
    msb: u8,
    /// Bank Select LSB.
    lsb: u8,
    /// Program Change (0-indexed).
    pc: u8,
    /// Tone name.
    name: String,
}

/// Local model for the tone selector modal.
#[derive(Lens)]
struct ToneSelectorData {
    /// Part index (0-15) being edited.
    part_index: usize,
    /// Currently active tone MSB.
    current_msb: u8,
    /// Currently active tone LSB.
    current_lsb: u8,
    /// Currently active tone PC.
    current_pc: u8,
    /// Selected bank group index.
    selected_group: usize,
    /// Selected bank index within the group.
    selected_bank: usize,
    /// Tones for the currently selected bank.
    tones: Vec<ToneDisplayEntry>,
}

impl ToneSelectorData {
    fn new(part_index: usize, current_msb: u8, current_lsb: u8, current_pc: u8) -> Self {
        // Find the initial bank from the current tone.
        let (group, bank) = find_bank_indices(current_msb, current_lsb).unwrap_or((0, 0));
        let mut data = Self {
            part_index,
            current_msb,
            current_lsb,
            current_pc,
            selected_group: group,
            selected_bank: bank,
            tones: Vec::new(),
        };
        data.load_tones();
        data
    }

    /// Load tones for the currently selected bank from the factory catalog.
    fn load_tones(&mut self) {
        self.tones.clear();
        let Some(bank) = self.current_bank_def() else {
            return;
        };
        let msb = bank.msb;
        let lsbs: Vec<u8> = bank.lsbs.to_vec();

        let mut number = 1;
        for lsb in lsbs {
            for tone in factory_tones(msb, lsb) {
                self.tones.push(ToneDisplayEntry {
                    number,
                    msb: tone.msb,
                    lsb: tone.lsb,
                    pc: tone.pc,
                    name: tone.name.to_string(),
                });
                number += 1;
            }
        }
    }

    /// Get the currently selected bank definition.
    fn current_bank_def(&self) -> Option<&'static ToneBank> {
        let group = TONE_BANK_GROUPS.get(self.selected_group)?;
        group.banks.get(self.selected_bank)
    }
}

impl Model for ToneSelectorData {
    fn event(&mut self, _cx: &mut EventContext, event: &mut Event) {
        event.map(|e, _| match e {
            ToneSelectorInternal::SelectBank(group_idx, bank_idx) => {
                self.selected_group = *group_idx;
                self.selected_bank = *bank_idx;
                self.load_tones();
            }
        });
    }
}

/// Find the group and bank indices for a given MSB/LSB pair.
fn find_bank_indices(msb: u8, lsb: u8) -> Option<(usize, usize)> {
    for (gi, group) in TONE_BANK_GROUPS.iter().enumerate() {
        for (bi, bank) in group.banks.iter().enumerate() {
            if bank.msb == msb && bank.lsbs.contains(&lsb) {
                return Some((gi, bi));
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// ToneSelector view
// ---------------------------------------------------------------------------

/// Modal overlay for selecting tones by bank.
///
/// Presents a two-panel layout: bank groups on the left, scrollable tone
/// list on the right. Closes on Escape or clicking the overlay background.
pub struct ToneSelector;

impl ToneSelector {
    /// Creates a new [`ToneSelector`] modal.
    ///
    /// - `part_index`: 0-based part number being edited.
    /// - `current_msb`, `current_lsb`, `current_pc`: the currently active tone.
    pub fn new(
        cx: &mut Context,
        part_index: usize,
        current_msb: u8,
        current_lsb: u8,
        current_pc: u8,
    ) -> Handle<'_, Self> {
        Self.build(cx, move |cx| {
            ToneSelectorData::new(part_index, current_msb, current_lsb, current_pc).build(cx);

            // Overlay background (click to close).
            // The overlay itself is the outer element; clicking it closes.
            // The modal stops propagation.

            // Modal container.
            VStack::new(cx, |cx| {
                // Header.
                HStack::new(cx, |cx| {
                    Binding::new(cx, ToneSelectorData::part_index, |cx, part_lens| {
                        let pi = part_lens.get(cx);
                        let title = format!("Tone Select — Part {}", pi + 1);
                        Label::new(cx, &title).class("tone-selector__title");
                    });

                    Element::new(cx).class("tone-selector__header-spacer");

                    Label::new(cx, "\u{2715}")
                        .class("tone-selector__close-btn")
                        .cursor(CursorIcon::Hand)
                        .on_press(|cx| {
                            cx.emit(ToneSelectorEvent::Close);
                        });
                })
                .class("tone-selector__header");

                // Body: two panels.
                HStack::new(cx, |cx| {
                    // Left panel: bank groups.
                    build_bank_list(cx);

                    // Right panel: tone list.
                    build_tone_list(cx);
                })
                .class("tone-selector__body");
            })
            .class("tone-selector__modal")
            .on_press(|_cx| {
                // Stop click propagation so clicking the modal doesn't close.
            });
        })
    }
}

impl View for ToneSelector {
    fn element(&self) -> Option<&'static str> {
        Some("tone-selector")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|e, _| {
            if let WindowEvent::KeyDown(Code::Escape, _) = e {
                cx.emit(ToneSelectorEvent::Close);
            }
        });
    }
}

/// Build the bank list panel (left side).
fn build_bank_list(cx: &mut Context) {
    // Combine group + bank selection into a single lens via map.
    let sel_lens = ToneSelectorData::selected_group.map(|g| *g);

    ScrollView::new(cx, 0.0, 0.0, false, true, move |cx| {
        VStack::new(cx, |cx| {
            // Rebuild the bank list whenever the selection changes.
            // We bind on selected_group and nest on selected_bank.
            Binding::new(cx, sel_lens, |cx, group_lens| {
                let sel_group = group_lens.get(cx);
                Binding::new(cx, ToneSelectorData::selected_bank, move |cx, bank_lens| {
                    let sel_bank = bank_lens.get(cx);
                    VStack::new(cx, |cx| {
                        for (gi, group) in TONE_BANK_GROUPS.iter().enumerate() {
                            Label::new(cx, group.label).class("tone-selector__group-label");
                            for (bi, bank) in group.banks.iter().enumerate() {
                                let is_active = gi == sel_group && bi == sel_bank;
                                Label::new(cx, bank.label)
                                    .class("tone-selector__bank-btn")
                                    .toggle_class("tone-selector__bank-btn--active", is_active)
                                    .cursor(CursorIcon::Hand)
                                    .on_press(move |cx| {
                                        cx.emit(ToneSelectorInternal::SelectBank(gi, bi));
                                    });
                            }
                        }
                    });
                });
            });
        });
    })
    .class("tone-selector__bank-list");
}

/// Build the tone list panel (right side).
fn build_tone_list(cx: &mut Context) {
    ScrollView::new(cx, 0.0, 0.0, false, true, |cx| {
        Binding::new(cx, ToneSelectorData::tones, |cx, tones_lens| {
            let tones = tones_lens.get(cx);
            VStack::new(cx, |cx| {
                if tones.is_empty() {
                    Label::new(cx, "No tones in this bank").class("tone-selector__empty");
                } else {
                    // Read current tone for highlighting via nested Bindings.
                    Binding::new(cx, ToneSelectorData::current_msb, move |cx, msb_lens| {
                        let cur_msb = msb_lens.get(cx);
                        let tones = tones.clone();
                        Binding::new(cx, ToneSelectorData::current_lsb, move |cx, lsb_lens| {
                            let cur_lsb = lsb_lens.get(cx);
                            let tones = tones.clone();
                            Binding::new(cx, ToneSelectorData::current_pc, move |cx, pc_lens| {
                                let cur_pc = pc_lens.get(cx);
                                VStack::new(cx, |cx| {
                                    for entry in &tones {
                                        let is_active = entry.msb == cur_msb
                                            && entry.lsb == cur_lsb
                                            && entry.pc == cur_pc;
                                        let msb = entry.msb;
                                        let lsb = entry.lsb;
                                        let pc = entry.pc;
                                        let display =
                                            format!("{:04}  {}", entry.number, entry.name);
                                        Label::new(cx, &display)
                                            .class("tone-selector__tone-item")
                                            .toggle_class(
                                                "tone-selector__tone-item--active",
                                                is_active,
                                            )
                                            .cursor(CursorIcon::Hand)
                                            .on_press(move |cx| {
                                                cx.emit(ToneSelectorEvent::SelectTone {
                                                    msb,
                                                    lsb,
                                                    pc,
                                                });
                                            });
                                    }
                                });
                            });
                        });
                    });
                }
            });
        });
    })
    .class("tone-selector__tone-list");
}
