import { hex4 } from './format';
import type { KeyName } from './ipc';

export type Size = 'Full size' | 'Tenkeyless' | '75%' | '65%' | '60%';
export type Std = 'ANSI' | 'ISO';

export const SIZES: readonly Size[] = ['Full size', 'Tenkeyless', '75%', '65%', '60%'];
export const STDS: readonly Std[] = ['ANSI', 'ISO'];

// A drawn key. Positions and sizes are in key units. The scan code is the key's position, never a
// character: the label is only what the drawing prints, and the name words the engine's sentences.
export interface Cap {
  scan: number | null;
  label: string;
  name: string;
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface Layout {
  size: Size;
  std: Std;
  caps: readonly Cap[];
  w: number;
  h: number;
  byScan: ReadonlyMap<number, Cap>;
}

// [scan, label, name]. Fn sends no code of its own, so it can't be tested or named in a finding.
type Key = readonly [number | null, string, string?];

const NUMBER_ROW: Key[] = [
  [0x29, '`'],
  [0x02, '1'],
  [0x03, '2'],
  [0x04, '3'],
  [0x05, '4'],
  [0x06, '5'],
  [0x07, '6'],
  [0x08, '7'],
  [0x09, '8'],
  [0x0a, '9'],
  [0x0b, '0'],
  [0x0c, '-'],
  [0x0d, '='],
];
const TOP_ROW: Key[] = [
  [0x10, 'Q'],
  [0x11, 'W'],
  [0x12, 'E'],
  [0x13, 'R'],
  [0x14, 'T'],
  [0x15, 'Y'],
  [0x16, 'U'],
  [0x17, 'I'],
  [0x18, 'O'],
  [0x19, 'P'],
  [0x1a, '['],
  [0x1b, ']'],
];
const HOME_ROW: Key[] = [
  [0x1e, 'A'],
  [0x1f, 'S'],
  [0x20, 'D'],
  [0x21, 'F'],
  [0x22, 'G'],
  [0x23, 'H'],
  [0x24, 'J'],
  [0x25, 'K'],
  [0x26, 'L'],
  [0x27, ';'],
  [0x28, "'"],
];
const BOTTOM_ROW: Key[] = [
  [0x2c, 'Z'],
  [0x2d, 'X'],
  [0x2e, 'C'],
  [0x2f, 'V'],
  [0x30, 'B'],
  [0x31, 'N'],
  [0x32, 'M'],
  [0x33, ','],
  [0x34, '.'],
  [0x35, '/'],
];
const F_KEYS: Key[] = [
  [0x3b, 'F1'],
  [0x3c, 'F2'],
  [0x3d, 'F3'],
  [0x3e, 'F4'],
  [0x3f, 'F5'],
  [0x40, 'F6'],
  [0x41, 'F7'],
  [0x42, 'F8'],
  [0x43, 'F9'],
  [0x44, 'F10'],
  [0x57, 'F11'],
  [0x58, 'F12'],
];

const ESC: Key = [0x01, 'Esc'];
const PRINT_SCREEN: Key = [0xe037, 'PrtSc', 'Print Screen'];
const SCROLL_LOCK: Key = [0x46, 'ScrLk', 'Scroll Lock'];
const PAUSE: Key = [0xe11d, 'Pause'];
const BACKSPACE: Key = [0x0e, 'Backspace'];
const TAB: Key = [0x0f, 'Tab'];
const CAPS_LOCK: Key = [0x3a, 'Caps', 'Caps Lock'];
const ENTER: Key = [0x1c, 'Enter'];
// ANSI's backslash and ISO's '#' share one code. ISO's backslash beside Left Shift has its own.
const ANSI_BACKSLASH: Key = [0x2b, '\\'];
const ISO_HASH: Key = [0x2b, '#'];
const ISO_BACKSLASH: Key = [0x56, '\\'];
const LEFT_SHIFT: Key = [0x2a, 'Shift', 'Left Shift'];
const RIGHT_SHIFT: Key = [0x36, 'Shift', 'Right Shift'];
const LEFT_CTRL: Key = [0x1d, 'Ctrl', 'Left Ctrl'];
const LEFT_WIN: Key = [0xe05b, 'Win', 'Left Win'];
const LEFT_ALT: Key = [0x38, 'Alt', 'Left Alt'];
const SPACE: Key = [0x39, '', 'Space'];
const RIGHT_ALT: Key = [0xe038, 'Alt', 'Right Alt'];
const FN: Key = [null, 'Fn'];
const MENU: Key = [0xe05d, 'Menu'];
const RIGHT_CTRL: Key = [0xe01d, 'Ctrl', 'Right Ctrl'];
const LEFT: Key = [0xe04b, '←', 'Left arrow'];
const DOWN: Key = [0xe050, '↓', 'Down arrow'];
const RIGHT: Key = [0xe04d, '→', 'Right arrow'];
const UP: Key = [0xe048, '↑', 'Up arrow'];
const INSERT: Key = [0xe052, 'Ins', 'Insert'];
const HOME: Key = [0xe047, 'Home'];
const PAGE_UP: Key = [0xe049, 'PgUp', 'Page Up'];
const PAGE_DOWN: Key = [0xe051, 'PgDn', 'Page Down'];
const END: Key = [0xe04f, 'End'];
const DELETE: Key = [0xe053, 'Del', 'Delete'];

const num = (scan: number, label: string): Key => [scan, label, 'Num ' + label];

// The digits, letters and punctuation, which the guided test prompts. The other keys act on the OS
// or the page, or send no code.
const PLAIN = new Set([
  ...[NUMBER_ROW, TOP_ROW, HOME_ROW, BOTTOM_ROW].flat().map(([scan]) => scan),
  ANSI_BACKSLASH[0],
  ISO_BACKSLASH[0],
]);

// A port of layout(size, std) in docs/design/keytriage demo.dc.html.
function build(size: Size, std: Std): Layout {
  const caps: Cap[] = [];
  const add = ([scan, label, name]: Key, x: number, y: number, w = 1, h = 1) =>
    caps.push({ scan, label, name: name ?? label, x, y, w, h });
  const row = (keys: Key[], x: number, y: number) => keys.forEach((key, i) => add(key, x + i, y));

  const tkl = size === 'Tenkeyless' || size === 'Full size';
  const compact = size === '75%' || size === '65%';
  const y0 = size === '75%' ? 1.25 : tkl ? 1.5 : 0;

  if (size === '75%') row([ESC, ...F_KEYS, PRINT_SCREEN, DELETE], 0, 0);
  if (tkl) {
    add(ESC, 0, 0);
    row(F_KEYS.slice(0, 4), 2, 0);
    row(F_KEYS.slice(4, 8), 6.5, 0);
    row(F_KEYS.slice(8), 11, 0);
    row([PRINT_SCREEN, SCROLL_LOCK, PAUSE], 15.25, 0);
  }

  row(NUMBER_ROW, 0, y0);
  add(BACKSPACE, 13, y0, 2);

  add(TAB, 0, y0 + 1, 1.5);
  row(TOP_ROW, 1.5, y0 + 1);
  if (std === 'ANSI') add(ANSI_BACKSLASH, 13.5, y0 + 1, 1.5);
  else add(ENTER, 13.75, y0 + 1, 1.25, 2);

  add(CAPS_LOCK, 0, y0 + 2, 1.75);
  row(HOME_ROW, 1.75, y0 + 2);
  if (std === 'ANSI') add(ENTER, 12.75, y0 + 2, 2.25);
  else add(ISO_HASH, 12.75, y0 + 2);

  if (std === 'ANSI') add(LEFT_SHIFT, 0, y0 + 3, 2.25);
  else {
    add(LEFT_SHIFT, 0, y0 + 3, 1.25);
    add(ISO_BACKSLASH, 1.25, y0 + 3);
  }
  row(BOTTOM_ROW, 2.25, y0 + 3);
  add(RIGHT_SHIFT, 12.25, y0 + 3, compact ? 1.75 : 2.75);

  add(LEFT_CTRL, 0, y0 + 4, 1.25);
  add(LEFT_WIN, 1.25, y0 + 4, 1.25);
  add(LEFT_ALT, 2.5, y0 + 4, 1.25);
  add(SPACE, 3.75, y0 + 4, 6.25);

  let w = 15;
  if (compact) {
    row([RIGHT_ALT, FN, RIGHT_CTRL, LEFT, DOWN, RIGHT], 10, y0 + 4);
    add(UP, 14, y0 + 3);
    add(size === '75%' ? HOME : DELETE, 15, y0);
    add(PAGE_UP, 15, y0 + 1);
    add(PAGE_DOWN, 15, y0 + 2);
    add(END, 15, y0 + 3);
    w = 16;
  } else {
    add(RIGHT_ALT, 10, y0 + 4, 1.25);
    add(FN, 11.25, y0 + 4, 1.25);
    add(MENU, 12.5, y0 + 4, 1.25);
    add(RIGHT_CTRL, 13.75, y0 + 4, 1.25);
  }

  if (tkl) {
    row([INSERT, HOME, PAGE_UP], 15.25, y0);
    row([DELETE, END, PAGE_DOWN], 15.25, y0 + 1);
    add(UP, 16.25, y0 + 3);
    row([LEFT, DOWN, RIGHT], 15.25, y0 + 4);
    w = 18.25;
  }

  if (size === 'Full size') {
    const n = 18.5;
    row([[0x45, 'Num', 'Num Lock'], num(0xe035, '/'), num(0x37, '*'), num(0x4a, '-')], n, y0);
    row([num(0x47, '7'), num(0x48, '8'), num(0x49, '9')], n, y0 + 1);
    add(num(0x4e, '+'), n + 3, y0 + 1, 1, 2);
    row([num(0x4b, '4'), num(0x4c, '5'), num(0x4d, '6')], n, y0 + 2);
    row([num(0x4f, '1'), num(0x50, '2'), num(0x51, '3')], n, y0 + 3);
    add(num(0xe01c, 'Enter'), n + 3, y0 + 3, 1, 2);
    add(num(0x52, '0'), n, y0 + 4, 2);
    add(num(0x53, '.'), n + 2, y0 + 4);
    w = 22.5;
  }

  const byScan = new Map<number, Cap>();
  for (const cap of caps) if (cap.scan !== null) byScan.set(cap.scan, cap);
  return { size, std, caps, w, h: y0 + 5, byScan };
}

const built = new Map<string, Layout>();

// The same size and standard always give the same object, so signals holding it stay equal.
export function layout(size: Size, std: Std): Layout {
  const key = `${size} ${std}`;
  let found = built.get(key);
  if (!found) built.set(key, (found = build(size, std)));
  return found;
}

export function keyCount(size: Size, std: Std): number {
  return layout(size, std).caps.length;
}

// Every plain key in reading order: row by row, left to right.
export function plainKeys(drawn: Layout): number[] {
  return drawn.caps
    .filter((cap): cap is Cap & { scan: number } => cap.scan !== null && PLAIN.has(cap.scan))
    .sort((a, b) => a.y - b.y || a.x - b.x)
    .map((cap) => cap.scan);
}

// The names end_test words its findings with. Keys the drawing lacks fall back to their code in Rust.
export function labelsFor(drawn: Layout): KeyName[] {
  return drawn.caps.flatMap((cap) =>
    cap.scan === null ? [] : [{ scan: cap.scan, name: cap.name }],
  );
}

export function capName(drawn: Layout, scan: number): string {
  return drawn.byScan.get(scan)?.name ?? hex4(scan);
}

// What a list or a tile prints for a key: its drawn label, its name when the cap is blank (Space),
// or its code when the layout doesn't draw it.
export function capLabel(drawn: Layout, scan: number): string {
  const cap = drawn.byScan.get(scan);
  return cap ? cap.label || cap.name : hex4(scan);
}
