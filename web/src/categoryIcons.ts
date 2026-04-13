/**
 * Simple monochrome SVG icons for INTEGRA-7 tone categories.
 *
 * ~15 icon families cover all 37 categories. Each returns a 24x24 SVG string.
 */

const S = 'xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"';

// Piano keys
const PIANO = `<svg ${S}><rect x="3" y="6" width="18" height="12" rx="1"/><line x1="8" y1="6" x2="8" y2="14"/><line x1="12" y1="6" x2="12" y2="14"/><line x1="16" y1="6" x2="16" y2="14"/><rect x="6" y="6" width="2.5" height="8" fill="currentColor" rx="0.5"/><rect x="10.5" y="6" width="2.5" height="8" fill="currentColor" rx="0.5"/><rect x="15" y="6" width="2.5" height="8" fill="currentColor" rx="0.5"/></svg>`;

// Keyboard / accordion
const KEYS = `<svg ${S}><rect x="2" y="7" width="20" height="10" rx="2"/><line x1="6" y1="7" x2="6" y2="17"/><line x1="10" y1="7" x2="10" y2="17"/><line x1="14" y1="7" x2="14" y2="17"/><line x1="18" y1="7" x2="18" y2="17"/></svg>`;

// Organ pipes
const ORGAN = `<svg ${S}><rect x="4" y="10" width="3" height="10" rx="1"/><rect x="8" y="6" width="3" height="14" rx="1"/><rect x="12" y="4" width="3" height="16" rx="1"/><rect x="16" y="8" width="3" height="12" rx="1"/></svg>`;

// Bell
const BELL = `<svg ${S}><path d="M12 3 v2"/><path d="M8 9 a4 4 0 0 1 8 0 v4 c0 2 1 3 2 4 H6 c1-1 2-2 2-4 z"/><circle cx="12" cy="20" r="1"/></svg>`;

// Guitar
const GUITAR = `<svg ${S}><ellipse cx="12" cy="16" rx="5" ry="4.5"/><ellipse cx="12" cy="16" rx="1.5" ry="1.2"/><path d="M17 16 V5 l3-2"/><line x1="7" y1="14" x2="7" y2="7"/></svg>`;

// Bass (thicker guitar)
const BASS = `<svg ${S}><ellipse cx="11" cy="16" rx="5.5" ry="4.5"/><ellipse cx="11" cy="16" rx="1.5" ry="1.2"/><path d="M16.5 14 V4 l2.5-1"/><circle cx="20" cy="5" r="1" fill="currentColor"/><circle cx="20" cy="8" r="1" fill="currentColor"/></svg>`;

// Plucked / harp
const PLUCK = `<svg ${S}><path d="M6 4 v16"/><path d="M6 4 Q18 4 18 20"/><line x1="6" y1="7" x2="16" y2="8"/><line x1="6" y1="10" x2="17" y2="12"/><line x1="6" y1="13" x2="17.5" y2="16"/></svg>`;

// Strings / violin
const STRINGS = `<svg ${S}><path d="M9 3 v18"/><path d="M15 3 v18"/><path d="M6 8 Q12 6 18 8"/><path d="M6 16 Q12 18 18 16"/><circle cx="8" cy="12" r="2"/><circle cx="16" cy="12" r="2"/></svg>`;

// Brass / trumpet
const BRASS = `<svg ${S}><path d="M3 10 h8 l2-2 h4 c2 0 4 2 4 4 s-2 4-4 4 h-4 l-2-2 H3 z"/><circle cx="5" cy="12" r="0.5" fill="currentColor"/><circle cx="7.5" cy="12" r="0.5" fill="currentColor"/><circle cx="10" cy="12" r="0.5" fill="currentColor"/></svg>`;

// Wind / flute / sax
const WIND = `<svg ${S}><path d="M4 8 Q8 4 12 8 Q16 12 20 8"/><path d="M4 12 Q8 8 12 12 Q16 16 20 12"/><path d="M4 16 Q8 12 12 16 Q16 20 20 16"/></svg>`;

// Voice / choir
const VOICE = `<svg ${S}><circle cx="12" cy="8" r="3"/><path d="M7 20 c0-4 2-6 5-6 s5 2 5 6"/><path d="M10 12 Q12 15 14 12"/></svg>`;

// Synth / waveform
const SYNTH = `<svg ${S}><polyline points="2,12 5,6 8,12 11,4 14,12 17,8 20,12 22,10"/><line x1="2" y1="18" x2="22" y2="18" stroke-dasharray="2 2"/></svg>`;

// Phrase / rhythm / sequence
const PHRASE = `<svg ${S}><rect x="3" y="14" width="3" height="6" rx="0.5"/><rect x="7.5" y="10" width="3" height="10" rx="0.5"/><rect x="12" y="6" width="3" height="14" rx="0.5"/><rect x="16.5" y="12" width="3" height="8" rx="0.5"/></svg>`;

// Drums
const DRUMS = `<svg ${S}><ellipse cx="12" cy="8" rx="8" ry="3"/><path d="M4 8 v6 c0 2 3.5 3 8 3 s8-1 8-3 V8"/><line x1="8" y1="3" x2="5" y2="8"/><line x1="16" y1="3" x2="19" y2="8"/></svg>`;

// Combination
const COMBO = `<svg ${S}><circle cx="8" cy="8" r="4"/><circle cx="16" cy="8" r="4"/><circle cx="12" cy="16" r="4"/></svg>`;

const ICON_MAP: Record<number, string> = {
  1: PIANO,    // Ac.Piano
  5: PIANO,    // E.Piano
  6: ORGAN,    // Organ
  10: KEYS,    // Other Keyboards
  12: KEYS,    // Accordion/Harmonica
  14: BELL,    // Bell/Mallet
  16: GUITAR,  // Ac.Guitar
  17: GUITAR,  // E.Guitar
  18: GUITAR,  // Dist.Guitar
  19: BASS,    // Ac.Bass
  20: BASS,    // E.Bass
  21: BASS,    // Synth Bass
  22: PLUCK,   // Plucked/Stroke
  24: STRINGS, // Strings
  26: BRASS,   // Brass
  28: WIND,    // Wind
  29: WIND,    // Flute
  30: WIND,    // Sax
  31: WIND,    // Recorder
  32: VOICE,   // Vox/Choir
  34: SYNTH,   // Synth Lead
  35: BRASS,   // Synth Brass
  36: SYNTH,   // Synth Pad/Strings
  37: SYNTH,   // Synth Bellpad
  38: SYNTH,   // Synth PolyKey
  39: SYNTH,   // FX
  40: SYNTH,   // Synth Seq/Pop
  41: PHRASE,  // Phrase
  42: PHRASE,  // Pulsating
  43: PHRASE,  // Beat&Groove
  44: PHRASE,  // Hit
  45: PHRASE,  // Sound FX
  46: DRUMS,   // Drums
  47: DRUMS,   // Percussion
  48: COMBO,   // Combination
};

/** Returns an SVG string (24x24) for the given category ID, or empty string. */
export function categoryIcon(categoryId: number): string {
  return ICON_MAP[categoryId] ?? "";
}
