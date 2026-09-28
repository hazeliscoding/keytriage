// The commands' and events' shapes. They mirror src-tauri/src/view.rs and session_core.rs.

export type Board = 'hot-swap' | 'soldered' | 'laptop';

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

export interface FindingView {
  key: number;
  kind: 'chatter' | 'dead' | 'stuck';
  confidence: 'low' | 'medium' | 'high' | 'very-high';
  title: string;
  level: string;
  strong: boolean;
  evidence: string[];
  causes: string[];
  next: string[];
  gaps: Bar[] | null;
}

export interface TestResult {
  rules: number;
  findings: FindingView[];
  notes: string[];
  clean: string[];
  keys: { scan: number; count: number }[];
  resolution: string;
}

export interface Events {
  'test:started': null;
  'test:event': TestEvent;
  'test:guide': GuideView;
  'test:stopped': null;
}
