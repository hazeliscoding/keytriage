import { type ComponentFixture } from '@angular/core/testing';
import { App } from './app';
import type { GuideView, TestEvent } from './ipc';
import {
  all,
  button,
  cap,
  click,
  el,
  inApp,
  key,
  leaveApp,
  render,
  send,
  sent,
  settle,
  testing,
  text,
  textOf,
} from './testing/harness';

const E = 0x12;
const W = 0x11;
const LEFT_ALT = 0x38;
const TAB = 0x0f;
// Handle 11 is the first keyboard, which the Start screen preselects. 21 is another keyboard.
const OWN = 11;
const OTHER = 21;
const G = 0x22;

const VIEW: GuideView = {
  key: E,
  asked: 10,
  count: 4,
  round: 0,
  rounds: 3,
  index: 0,
  keys: 12,
  done: 4,
  total: 360,
  tallies: [[E, 4]],
};
const PAUSED: TestEvent = { kind: 'paused', micros: 5_000_000, interrupted: [] };
const RESUMED: TestEvent = { kind: 'resumed', micros: 9_000_000 };

function cells(fixture: ComponentFixture<App>): string[][] {
  return all(fixture, '.live__row').map((row) => [...row.children].map((cell) => text(cell)));
}

function state(fixture: ComponentFixture<App>, scan: number): string {
  return cap(fixture, scan).className;
}

function count(fixture: ComponentFixture<App>, scan: number): string {
  return text(cap(fixture, scan).querySelector('.cap__count'));
}

function progress(fixture: ComponentFixture<App>): string[][] {
  return all(fixture, '.progress__line').map((line) =>
    [...line.children].map((cell) => text(cell)),
  );
}

function labels(fixture: ComponentFixture<App>): string[] {
  return all(fixture, '.footer button').map((b) => text(b));
}

describe('Test screen', () => {
  beforeEach(() => inApp());
  afterEach(leaveApp);

  describe('live events', () => {
    it('keeps the 16 newest rows on screen, newest first', async () => {
      const fixture = await testing();
      const events: TestEvent[] = [];
      for (let i = 0; i < 20; i++) events.push(key(W, i % 2 === 1, OWN, (i + 1) * 100_000));
      await send(fixture, 'test:event', ...events);
      const times = cells(fixture).map((row) => row[2]);
      expect(times).toHaveLength(16);
      expect(times[0]).toBe('2 000.0');
      expect(times[15]).toBe('500.0');
    });

    it("lists a key from the tested keyboard and draws it down until it's released", async () => {
      const fixture = await testing();
      await send(fixture, 'test:event', key(E, false, OWN, 1_500));
      expect(cells(fixture)).toEqual([['E', 'down', '1.5', '046D:C52B']]);
      expect(state(fixture, E)).toBe('cap cap--down');
      await send(fixture, 'test:event', key(E, true, OWN, 61_500));
      expect(cells(fixture)[0]).toEqual(['E', 'up', '61.5', '046D:C52B']);
      expect(state(fixture, E)).toBe('cap');
    });

    it('lists a key from another keyboard without drawing it', async () => {
      const fixture = await testing();
      await send(fixture, 'test:event', key(E, false, OTHER, 1_500), key(W, false, 99, 2_000));
      expect(cells(fixture)).toEqual([
        ['W', 'down', '2.0', '—'],
        ['E', 'down', '1.5', '3434:0220'],
      ]);
      expect(state(fixture, E)).toBe('cap');
      expect(state(fixture, W)).toBe('cap');
    });

    it('marks injected events, says how many arrived, and never draws them', async () => {
      const fixture = await testing();
      await send(fixture, 'test:event', key(E, false, 0, 1_000), key(E, true, 0, 51_000));
      const rows = all(fixture, '.live__row');
      expect(cells(fixture).map((row) => row[3])).toEqual(['injected', 'injected']);
      expect(rows.every((row) => row.classList.contains('live__row--injected'))).toBe(true);
      expect(textOf(fixture, '.notice__label')).toBe('INJECTED INPUT');
      expect(textOf(fixture, '.notice__body')).toMatch(
        /^2 events did not come from a physical keyboard\. They stay in the event list, marked injected, and are left out of every count\./,
      );
      expect(state(fixture, E)).toBe('cap');
      await send(fixture, 'test:event', key(E, false, 0, 90_000));
      expect(state(fixture, E)).toBe('cap');
    });

    it('takes no events before a test or after it ends', async () => {
      const fixture = await render();
      await send(fixture, 'test:event', key(E, false, OWN, 1_000));
      await click(fixture, button(fixture, 'Begin test'));
      expect(all(fixture, '.live__row')).toHaveLength(0);
      await click(fixture, button(fixture, 'End test'));
      expect(sent('end_test')).toHaveLength(1);
      expect(textOf(fixture, '.steps__now')).toBe('03 Findings');
      await send(fixture, 'test:event', key(E, false, OWN, 2_000));
      await click(fixture, button(fixture, 'New test'));
      await click(fixture, button(fixture, 'Begin test'));
      expect(all(fixture, '.live__row')).toHaveLength(0);
    });
  });

  describe('a focus loss', () => {
    const paused: TestEvent = {
      kind: 'paused',
      micros: 131_100_000,
      interrupted: [
        { device: OWN, scan: LEFT_ALT },
        { device: OWN, scan: TAB },
        { device: OTHER, scan: W },
      ],
    };

    it('pauses with the time, marks the keys that were down as interrupted, and resumes', async () => {
      const fixture = await testing();
      await send(fixture, 'test:event', key(LEFT_ALT, false, OWN, 130_962_100), paused);
      expect(textOf(fixture, '.notice--attention .notice__label')).toMatch(
        /^PAUSED · WINDOW LOST FOCUS \d{2}:\d{2}$/,
      );
      expect(textOf(fixture, '.notice--attention .notice__body')).toBe(
        'Click back into the window to continue. Alt and Tab were down at that moment and are ' +
          'marked interrupted, not stuck.',
      );
      expect(state(fixture, LEFT_ALT)).toBe('cap cap--interrupted');
      expect(state(fixture, TAB)).toBe('cap cap--interrupted');
      expect(text(cap(fixture, TAB).querySelector('.cap__tag'))).toBe('○');
      expect(state(fixture, W)).toBe('cap');
      const [lost] = cells(fixture);
      expect(lost.slice(0, 2)).toEqual(['—', 'focus lost']);
      expect(lost[2]).toMatch(/^\d{2}:\d{2}$/);
      expect(lost[3]).toBe('window');
      expect(all(fixture, '.live__row')[0].classList).toContain('live__row--window');
      expect(textOf(fixture, '.header__capture')).toBe('○ Paused');

      await send(fixture, 'test:event', { kind: 'resumed', micros: 140_000_000 });
      expect(all(fixture, '.notice')).toHaveLength(0);
      expect(state(fixture, LEFT_ALT)).toBe('cap');
      expect(state(fixture, TAB)).toBe('cap');
      expect(textOf(fixture, '.header__capture')).toBe('● Capturing');
    });

    it('names one interrupted key, or none', async () => {
      const fixture = await testing();
      await send(fixture, 'test:event', { ...paused, interrupted: [{ device: OWN, scan: TAB }] });
      expect(textOf(fixture, '.notice__body')).toContain(
        'Tab was down at that moment and is marked interrupted, not stuck.',
      );
      await send(
        fixture,
        'test:event',
        { kind: 'resumed', micros: 1 },
        { ...paused, interrupted: [] },
      );
      expect(textOf(fixture, '.notice__body')).toContain('No key was down at that moment.');
    });

    it('tells keys with the same label apart by name', async () => {
      const fixture = await testing();
      const both = [
        { device: OWN, scan: LEFT_ALT },
        { device: OWN, scan: 0xe038 },
        { device: OWN, scan: TAB },
      ];
      await send(fixture, 'test:event', { ...paused, interrupted: both });
      expect(textOf(fixture, '.notice__body')).toContain(
        'Left Alt, Right Alt and Tab were down at that moment',
      );
    });

    it('hides the injected notice while paused', async () => {
      const fixture = await testing();
      await send(fixture, 'test:event', key(E, false, 0, 1_000), paused);
      expect(all(fixture, '.notice').map((n) => text(n.querySelector('.notice__label')))).toEqual([
        expect.stringMatching(/^PAUSED/),
      ]);
    });
  });

  describe('keys', () => {
    it('keeps focus off the page at Begin and after each click', async () => {
      const fixture = await render();
      const begin = button(fixture, 'Begin test');
      begin.focus();
      await click(fixture, begin);
      expect(document.activeElement).toBe(document.body);
      const end = button(fixture, 'End test');
      end.focus();
      end.dispatchEvent(new MouseEvent('click', { bubbles: true }));
      expect(document.activeElement).not.toBe(end);
      await settle(fixture);
    });
  });

  describe('the prompt', () => {
    it('asks for the key Rust prompts, with its count and the progress', async () => {
      const fixture = await testing();
      await send(fixture, 'test:guide', VIEW);
      expect(textOf(fixture, '.prompt__key')).toBe('E');
      expect(textOf(fixture, '.prompt__ask')).toBe('Press E ten times.');
      expect(textOf(fixture, '.prompt__sub')).toBe(
        'Normal pace. Release fully between presses. Other keys are ignored.',
      );
      expect(textOf(fixture, '.prompt__count')).toBe('4 / 10');
      expect(progress(fixture)).toEqual([
        ['Round 1 of 3', 'Key 1 of 12'],
        ['Elapsed 00:00', '4 of 360 presses'],
      ]);
      const fill = el(fixture).querySelector<HTMLElement>('.progress__fill')?.style.width ?? '';
      expect(parseFloat(fill)).toBeCloseTo((4 / 360) * 100);
      expect(state(fixture, E)).toBe('cap cap--prompted');
      expect(count(fixture, E)).toBe('4');
    });

    it('follows each view Rust sends, with thousands grouped', async () => {
      const fixture = await testing();
      await send(fixture, 'test:guide', VIEW, {
        ...VIEW,
        key: G,
        count: 0,
        round: 1,
        index: 3,
        keys: 47,
        done: 120,
        total: 1410,
      });
      expect(textOf(fixture, '.prompt__ask')).toBe('Press G ten times.');
      expect(textOf(fixture, '.prompt__count')).toBe('0 / 10');
      expect(progress(fixture)).toEqual([
        ['Round 2 of 3', 'Key 4 of 47'],
        ['Elapsed 00:00', '120 of 1 410 presses'],
      ]);
    });

    it('asks once for a single press', async () => {
      const fixture = await testing();
      await send(fixture, 'test:guide', { ...VIEW, asked: 1, count: 0 });
      expect(textOf(fixture, '.prompt__ask')).toBe('Press E once.');
    });

    it('draws each counted key with its tally, and the prompted key over them', async () => {
      const fixture = await testing();
      await send(fixture, 'test:guide', {
        ...VIEW,
        key: G,
        tallies: [
          [E, 6],
          [G, 0],
        ],
      });
      expect(state(fixture, E)).toBe('cap cap--counted');
      expect(count(fixture, E)).toBe('6');
      expect(state(fixture, G)).toBe('cap cap--prompted');
      expect(count(fixture, G)).toBe('');
      expect(state(fixture, W)).toBe('cap');
    });

    it('draws a held key as down and an interrupted key as interrupted, keeping counts', async () => {
      const fixture = await testing();
      await send(fixture, 'test:guide', VIEW);
      await send(fixture, 'test:event', key(E, false, OWN, 1_000_000));
      expect(state(fixture, E)).toBe('cap cap--down');
      expect(count(fixture, E)).toBe('4');
      await send(fixture, 'test:event', { ...PAUSED, interrupted: [{ device: OWN, scan: E }] });
      expect(state(fixture, E)).toBe('cap cap--interrupted');
      expect(count(fixture, E)).toBe('4');
      expect(textOf(fixture, '.notice__body')).toBe(
        'Click back into the window to continue. E was down at that moment and is marked ' +
          'interrupted, not stuck. This round of E will be repeated.',
      );
    });

    it('counts injected events in place of the presses', async () => {
      const fixture = await testing();
      await send(fixture, 'test:guide', VIEW);
      await send(fixture, 'test:event', key(E, false, 0, 1_000), key(E, true, 0, 51_000));
      expect(progress(fixture)[1]).toEqual(['Elapsed 00:00', '2 injected, not counted']);
      expect(textOf(fixture, '.prompt__count')).toBe('4 / 10');
    });
  });

  describe('controls', () => {
    it("pauses on request, reads the pause as the user's own, and continues", async () => {
      const fixture = await testing();
      await send(fixture, 'test:guide', VIEW);
      expect(labels(fixture)).toEqual(['Pause', 'Skip this key', 'End test']);
      await click(fixture, button(fixture, 'Pause'));
      expect(sent('pause_test')).toEqual([{}]);
      await send(fixture, 'test:event', PAUSED);
      expect(textOf(fixture, '.notice__label')).toBe('PAUSED');
      expect(textOf(fixture, '.notice__body')).toBe(
        'Continue when ready. No key was down at that moment. This round of E will be repeated.',
      );
      expect(all(fixture, '.live__row--window')).toHaveLength(0);
      expect(el(fixture).querySelector('.prompt')?.classList).toContain('prompt--paused');
      expect(textOf(fixture, '.prompt__sub')).toBe('Waiting for focus. Presses are not read.');
      expect(el(fixture).querySelector('.progress')?.classList).toContain('progress--paused');
      expect(progress(fixture)[1][0]).toBe('Elapsed 00:00, paused');
      expect(textOf(fixture, '.header__capture')).toBe('○ Paused');
      expect(labels(fixture)).toEqual(['Continue', 'Skip this key', 'End test']);
      expect(button(fixture, 'Continue').classList).toContain('btn--primary');

      await click(fixture, button(fixture, 'Continue'));
      expect(sent('continue_test')).toEqual([{}]);
      await send(fixture, 'test:event', RESUMED);
      expect(labels(fixture)).toEqual(['Pause', 'Skip this key', 'End test']);
      expect(el(fixture).querySelector('.prompt')?.classList).not.toContain('prompt--paused');
    });

    it('reads a later pause without a click as a focus loss', async () => {
      const fixture = await testing();
      await send(fixture, 'test:guide', VIEW);
      await click(fixture, button(fixture, 'Pause'));
      await send(fixture, 'test:event', PAUSED, RESUMED, PAUSED);
      expect(textOf(fixture, '.notice__label')).toMatch(/^PAUSED · WINDOW LOST FOCUS \d{2}:\d{2}$/);
      expect(all(fixture, '.live__row--window')).toHaveLength(1);
    });

    it('skips the prompted key', async () => {
      const fixture = await testing();
      await send(fixture, 'test:guide', VIEW);
      await click(fixture, button(fixture, 'Skip this key'));
      expect(sent('skip_key')).toEqual([{}]);
    });

    it('gives the reason a command failed and stays on the test', async () => {
      inApp((cmd) => {
        if (cmd === 'pause_test' || cmd === 'skip_key') throw 'No test is running.';
        return null;
      });
      const fixture = await testing();
      await send(fixture, 'test:guide', VIEW);
      await click(fixture, button(fixture, 'Skip this key'));
      expect(textOf(fixture, '.footer__note')).toBe('No test is running.');
      expect(textOf(fixture, '.steps__now')).toBe('02 Test');
      await click(fixture, button(fixture, 'Pause'));
      await send(fixture, 'test:event', PAUSED);
      expect(textOf(fixture, '.notice__label')).toMatch(/^PAUSED · WINDOW LOST FOCUS/);
    });
  });

  describe('elapsed clock', () => {
    afterEach(() => vi.useRealTimers());

    it('ticks each second and stands still while paused', async () => {
      vi.useFakeTimers({ toFake: ['Date', 'setInterval', 'clearInterval'] });
      const fixture = await testing();
      await send(fixture, 'test:guide', VIEW);
      const elapsed = async (ms: number) => {
        vi.advanceTimersByTime(ms);
        await fixture.whenStable();
        return progress(fixture)[1][0];
      };
      expect(await elapsed(3_000)).toBe('Elapsed 00:03');
      await send(fixture, 'test:event', PAUSED);
      expect(await elapsed(5_000)).toBe('Elapsed 00:03, paused');
      await send(fixture, 'test:event', RESUMED);
      expect(await elapsed(2_000)).toBe('Elapsed 00:05');
    });
  });
});
