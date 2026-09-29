// The commands' and events' shapes. They mirror src-tauri/src/view.rs and session_core.rs.

export type Board = 'hot-swap' | 'soldered' | 'laptop';

// start_test's refusal when the picked keyboard's handles are gone. Mirrors view.rs.
export const RECONNECTED =
  'This keyboard was unplugged or reconnected. Pick it again on the Start screen.';

export interface PlanArgs {
  keyboard: number[];
  keys: number[];
  rounds: number;
  presses: number;
  board: Board | null;
}

// Scan codes are set-1 make codes with 0xE0 or 0xE1 in the high byte. Device 0 is injected input.
export type TestEvent =
  | { kind: 'key'; scan: number; up: boolean; device: number; micros: number }
  | { kind: 'paused'; micros: number; interrupted: { device: number; scan: number }[] }
  | { kind: 'resumed'; micros: number };

export interface GuideView {
  key: number | null;
  asked: number;
  count: number;
  round: number;
  rounds: number;
  index: number;
  keys: number;
  done: number;
  total: number;
  tallies: [number, number][];
  // Only once the plan is done: how long to wait before ending the test.
  waitMs?: number;
}

export interface KeyboardEntry {
  handle: number;
  name: string;
  id: string | null;
}

export interface KeyboardGroup {
  name: string;
  id: string | null;
  builtIn: boolean;
  entries: KeyboardEntry[];
}

export interface KeyName {
  scan: number;
  name: string;
}

export interface Bar {
  label: string;
  count: number;
}

export type Kind = 'chatter' | 'dead' | 'stuck';
export type Level = 'low' | 'medium' | 'high' | 'very-high';
export type Outcome = 'follows' | 'stays' | 'both' | 'gone' | 'unclear';

export interface FindingView {
  key: number;
  kind: Kind;
  confidence: Level;
  title: string;
  level: string;
  strong: boolean;
  evidence: string[];
  causes: string[];
  next: string[];
  gaps: Bar[] | null;
}

export interface SwapView {
  suspect: number;
  partner: number;
  title: string;
  knownGood: string;
  steps: string[];
  means: string;
  note: string;
  // The test never prompted the partner, so its switch isn't known to be good.
  partnerUntested: boolean;
}

export interface OutcomeView {
  outcome: Outcome;
  tile: number | null;
  flagged: number[];
  title: string;
  // Null only for an unclear outcome, which the engine gives no confidence.
  confidence: Level | null;
  level: string | null;
  strong: boolean;
  evidence: string[];
  diagnosis: string;
  next: string[];
}

export interface TestResult {
  rules: number;
  // Empty after a swap test, whose own swap steps would mislead.
  findings: FindingView[];
  notes: string[];
  clean: string[];
  keys: { scan: number; count: number }[];
  resolution: string;
  // Rust offers a swap only on a hot-swap board, for a finding that names a swap partner.
  swap: SwapView | null;
  // Set only after a swap test.
  outcome: OutcomeView | null;
}

export interface Events {
  'test:started': null;
  'test:event': TestEvent;
  'test:guide': GuideView;
  'test:stopped': null;
}
