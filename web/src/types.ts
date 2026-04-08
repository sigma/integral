/** State of a single Part in the mixer. */
export interface PartState {
  /** Part level / volume (0–127). */
  level: number;
  /** Part pan (0–127, 64 = center). */
  pan: number;
  /** Whether the part is muted. */
  muted: boolean;
  /** Tone bank MSB. */
  toneBankMsb: number;
  /** Tone bank LSB. */
  toneBankLsb: number;
  /** Tone program number. */
  tonePC: number;
  /** MIDI receive channel (0–15). */
  receiveChannel: number;
  /** Tone name read from the device. */
  toneName: string;
}

/** Full mixer state. */
export interface MixerState {
  /** Studio Set name (up to 16 ASCII chars). */
  studioSetName: string;
  /** Current Studio Set PC (0–63). */
  studioSetPC: number;
  /** System master level (0–127). */
  masterLevel: number;
  /** All 16 parts. */
  parts: PartState[];
  /** Currently selected part index (0–15). */
  selectedPart: number;
  /** Whether initial state is still loading from the device. */
  loading: boolean;
  /** All 64 Studio Set names (indexed 0–63). Populated via catalog query. */
  studioSetNames: Map<number, string>;
}

/** Default state for a single part. */
export function defaultPartState(): PartState {
  return {
    level: 100,
    pan: 64,
    muted: false,
    toneBankMsb: 0,
    toneBankLsb: 0,
    tonePC: 0,
    receiveChannel: 0,
    toneName: "",
  };
}

/** Default mixer state (before device sync). */
export function defaultMixerState(): MixerState {
  return {
    studioSetName: "",
    studioSetPC: 0,
    masterLevel: 100,
    parts: Array.from({ length: 16 }, () => defaultPartState()),
    selectedPart: 0,
    loading: true,
    studioSetNames: new Map(),
  };
}
