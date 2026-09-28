import { clock, countWord, fileName, fmtMicros, grouped, hex4, mmss, stamp } from './format';

describe('format', () => {
  it('prints microseconds as milliseconds, truncated to 0.1 ms', () => {
    expect(fmtMicros(128_466_345)).toBe('128 466.3');
    expect(fmtMicros(999_960)).toBe('999.9');
    expect(fmtMicros(1_500)).toBe('1.5');
    expect(fmtMicros(99)).toBe('0.0');
    expect(fmtMicros(1_234_567_890)).toBe('1 234 567.8');
  });

  it('groups thousands with a space', () => {
    expect(grouped(0)).toBe('0');
    expect(grouped(999)).toBe('999');
    expect(grouped(1_410)).toBe('1 410');
    expect(grouped(1_234_567)).toBe('1 234 567');
  });

  it('prints minutes and seconds', () => {
    expect(mmss(0)).toBe('00:00');
    expect(mmss(372)).toBe('06:12');
    expect(mmss(372.9)).toBe('06:12');
    expect(mmss(4_325)).toBe('72:05');
  });

  it('prints dates and times from the page clock', () => {
    const at = new Date(2026, 8, 4, 8, 5, 59);
    expect(stamp(at)).toBe('2026.09.04 08:05');
    expect(clock(at)).toBe('08:05');
    expect(clock(new Date(2026, 8, 24, 18, 42))).toBe('18:42');
  });

  it('prints codes as four hex digits', () => {
    expect(hex4(0x12)).toBe('0012');
    expect(hex4(0xe048)).toBe('E048');
    expect(hex4(0xe11d)).toBe('E11D');
  });

  it('spells counts up to twenty', () => {
    expect(countWord(1)).toBe('one');
    expect(countWord(10)).toBe('ten');
    expect(countWord(20)).toBe('twenty');
    expect(countWord(30)).toBe('30');
  });

  it('names the report file', () => {
    const name = fileName(new Date(2026, 8, 28, 14, 2));
    expect(name).toBe('keytriage-2026.09.28-1402.json');
    expect(name).toMatch(/^keytriage-\d{4}\.\d{2}\.\d{2}-\d{4}\.json$/);
  });
});
