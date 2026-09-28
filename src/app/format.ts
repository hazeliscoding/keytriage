export const pad2 = (n: number) => String(n).padStart(2, '0');

const WORDS = [
  'one',
  'two',
  'three',
  'four',
  'five',
  'six',
  'seven',
  'eight',
  'nine',
  'ten',
  'eleven',
  'twelve',
  'thirteen',
  'fourteen',
  'fifteen',
  'sixteen',
  'seventeen',
  'eighteen',
  'nineteen',
  'twenty',
];

// A whole number with a space between thousands: 1 410.
export function grouped(n: number): string {
  return String(Math.trunc(n)).replace(/\B(?=(\d{3})+(?!\d))/g, ' ');
}

// Microseconds as milliseconds, truncated to 0.1 ms, so a reading never looks later than it was.
export function fmtMicros(micros: number): string {
  const tenths = Math.floor(micros / 100);
  return `${grouped(Math.floor(tenths / 10))}.${tenths % 10}`;
}

export function mmss(seconds: number): string {
  const whole = Math.floor(seconds);
  return `${pad2(Math.floor(whole / 60))}:${pad2(whole % 60)}`;
}

function day(date: Date): string {
  return `${date.getFullYear()}.${pad2(date.getMonth() + 1)}.${pad2(date.getDate())}`;
}

export function clock(date: Date): string {
  return `${pad2(date.getHours())}:${pad2(date.getMinutes())}`;
}

export function stamp(date: Date): string {
  return `${day(date)} ${clock(date)}`;
}

export function hex4(scan: number): string {
  return scan.toString(16).toUpperCase().padStart(4, '0');
}

export function countWord(n: number): string {
  return WORDS[n - 1] ?? String(n);
}

// Rust refuses any other name, so the page can't put anything else into the file.
export function fileName(date: Date): string {
  return `keytriage-${day(date)}-${pad2(date.getHours())}${pad2(date.getMinutes())}.json`;
}
