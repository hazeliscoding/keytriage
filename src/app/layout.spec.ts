import {
  SIZES,
  STDS,
  capLabel,
  capName,
  keyCount,
  labelsFor,
  layout,
  plainKeys,
  type Cap,
  type Layout,
} from './layout';
import { DEFAULT_PLAN } from './plan';

const ALL: Layout[] = SIZES.flatMap((size) => STDS.map((std) => layout(size, std)));

const E = 0x12;
const G = 0x22;
const J = 0x24;

function overlaps(a: Cap, b: Cap): boolean {
  return a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;
}

function labelOf(drawn: Layout, scan: number): string | undefined {
  return drawn.caps.find((cap) => cap.scan === scan)?.label;
}

describe('layout', () => {
  it('draws the key counts of each size', () => {
    const ansi = SIZES.map((size) => keyCount(size, 'ANSI'));
    const iso = SIZES.map((size) => keyCount(size, 'ISO'));
    expect(ansi).toEqual([104, 87, 83, 68, 61]);
    expect(iso).toEqual([105, 88, 84, 69, 62]);
  });

  it('gives every key but Fn one code of its own', () => {
    for (const drawn of ALL) {
      const missing = drawn.caps.filter((cap) => cap.scan === null).map((cap) => cap.label);
      expect(missing).toEqual(['Fn']);
      const scans = drawn.caps.flatMap((cap) => (cap.scan === null ? [] : [cap.scan]));
      expect(new Set(scans).size).toBe(scans.length);
      expect(drawn.byScan.size).toBe(scans.length);
    }
  });

  it('keeps every key inside the board without overlaps', () => {
    for (const drawn of ALL) {
      for (const cap of drawn.caps) {
        expect(cap.x).toBeGreaterThanOrEqual(0);
        expect(cap.y).toBeGreaterThanOrEqual(0);
        expect(cap.x + cap.w).toBeLessThanOrEqual(drawn.w);
        expect(cap.y + cap.h).toBeLessThanOrEqual(drawn.h);
      }
      drawn.caps.forEach((a, i) =>
        drawn.caps.slice(i + 1).forEach((b) => {
          if (overlaps(a, b))
            throw new Error(`${drawn.size} ${drawn.std}: ${a.name} overlaps ${b.name}`);
        }),
      );
    }
  });

  it('uses the design aspect ratios', () => {
    const sizes = SIZES.map((size) => layout(size, 'ANSI')).map(({ w, h }) => [w, h]);
    expect(sizes).toEqual([
      [22.5, 6.5],
      [18.25, 6.5],
      [16, 6.25],
      [16, 5],
      [15, 5],
    ]);
  });

  it('codes the backslash and hash keys by standard', () => {
    for (const size of SIZES) {
      const ansi = layout(size, 'ANSI');
      const iso = layout(size, 'ISO');
      expect(labelOf(ansi, 0x2b)).toBe('\\');
      expect(labelOf(ansi, 0x56)).toBeUndefined();
      expect(labelOf(iso, 0x2b)).toBe('#');
      expect(labelOf(iso, 0x56)).toBe('\\');
    }
  });

  it('draws prefixed keys with their prefix in the high byte', () => {
    const full = layout('Full size', 'ANSI');
    expect(capName(full, 0xe048)).toBe('Up arrow');
    expect(capName(full, 0xe053)).toBe('Delete');
    expect(capName(full, 0xe01c)).toBe('Num Enter');
    expect(capName(full, 0x1c)).toBe('Enter');
    expect(capName(full, 0xe11d)).toBe('Pause');
  });

  it('names every key once', () => {
    for (const drawn of ALL) {
      const names = drawn.caps.map((cap) => cap.name);
      expect(new Set(names).size).toBe(names.length);
      for (const name of names) expect(name.length).toBeLessThanOrEqual(24);
    }
  });

  it('prompts every plain key in reading order', () => {
    for (const drawn of ALL) {
      const keys = DEFAULT_PLAN.keys(drawn);
      expect(keys.length).toBe(drawn.std === 'ANSI' ? 47 : 48);
      expect(new Set(keys).size).toBe(keys.length);
      for (const key of [...keys, G, J, E]) expect(drawn.byScan.has(key)).toBe(true);
    }
    const ansi = plainKeys(layout('75%', 'ANSI')).map((scan) =>
      capLabel(layout('75%', 'ANSI'), scan),
    );
    expect(ansi.join('')).toBe("`1234567890-=QWERTYUIOP[]\\ASDFGHJKL;'ZXCVBNM,./");
    const iso = plainKeys(layout('60%', 'ISO')).map((scan) => capLabel(layout('60%', 'ISO'), scan));
    expect(iso.join('')).toBe("`1234567890-=QWERTYUIOP[]ASDFGHJKL;'#\\ZXCVBNM,./");
  });

  it('tests 3 rounds of 10 presses by default', () => {
    expect(DEFAULT_PLAN.rounds).toBe(3);
    expect(DEFAULT_PLAN.presses).toBe(10);
  });

  it('names keys for the engine', () => {
    for (const drawn of ALL) {
      const labels = labelsFor(drawn);
      expect(labels.length).toBe(drawn.caps.length - 1);
      const name = (scan: number) => labels.find((label) => label.scan === scan)?.name;
      expect([name(E), name(G), name(J)]).toEqual(['E', 'G', 'J']);
    }
    const tkl = labelsFor(layout('Tenkeyless', 'ISO'));
    expect(tkl).toContainEqual({ scan: 0x2a, name: 'Left Shift' });
    expect(tkl).toContainEqual({ scan: 0x39, name: 'Space' });
  });

  it('prints a label, the name for a blank cap, or the code', () => {
    const drawn = layout('75%', 'ANSI');
    expect(capLabel(drawn, 0x38)).toBe('Alt');
    expect(capLabel(drawn, 0x39)).toBe('Space');
    expect(capLabel(drawn, 0xe05c)).toBe('E05C');
    expect(capName(drawn, 0x45)).toBe('0045');
  });

  it('returns the same layout for the same choice', () => {
    expect(layout('65%', 'ISO')).toBe(layout('65%', 'ISO'));
  });
});
