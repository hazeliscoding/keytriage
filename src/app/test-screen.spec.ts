import { type ComponentFixture } from '@angular/core/testing';
import { App } from './app';
import type { GuideView, TestEvent, TestResult } from './ipc';
import {
  EMPTY,
  FOLLOWS,
  GOLDEN,
  all,
  button,
  cap,
  click,
  el,
  inApp,
  key,
  leaveApp,
  nthClick,
  offered,
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
        // Every closed step counts in full: 47 keys in round 1, then 3 in round 2.
        done: 500,
        total: 1410,
      });
      expect(textOf(fixture, '.prompt__ask')).toBe('Press G ten times.');
      expect(textOf(fixture, '.prompt__count')).toBe('0 / 10');
      expect(progress(fixture)).toEqual([
        ['Round 2 of 3', 'Key 4 of 47'],
        ['Elapsed 00:00', '500 of 1 410 presses'],
      ]);
    });

    it('asks once for a single press', async () => {
      const fixture = await testing();
      await send(fixture, 'test:guide', { ...VIEW, asked: 1, count: 0 });
      expect(textOf(fixture, '.prompt__ask')).toBe('Press E once.');
    });

    it('names a key whose label is shared or blank, and keeps the label on the tile', async () => {
      const fixture = await testing();
      const ask = async (scan: number) => {
        await send(fixture, 'test:guide', { ...VIEW, key: scan, asked: 30, count: 0 });
        return [textOf(fixture, '.prompt__key'), textOf(fixture, '.prompt__ask')];
      };
      expect(await ask(0xe038)).toEqual(['Alt', 'Press Right Alt 30 times.']);
      expect(await ask(0x39)).toEqual(['Space', 'Press Space 30 times.']);
      expect(await ask(0xe049)).toEqual(['PgUp', 'Press PgUp 30 times.']);
    });

    it('names the round a pause repeats as the prompt does', async () => {
      const fixture = await testing();
      await send(fixture, 'test:guide', { ...VIEW, key: 0xe038, asked: 30 });
      await send(fixture, 'test:event', PAUSED);
      expect(textOf(fixture, '.notice__body')).toMatch(
        / This round of Right Alt will be repeated\.$/,
      );
    });

    it('draws each counted key with its tally, and the prompted key over them', async () => {
      const fixture = await testing();
      await send(fixture, 'test:guide', { ...VIEW, key: G, tallies: [[E, 6]] });
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

  describe('announcements', () => {
    const region = (fixture: ComponentFixture<App>) =>
      el(fixture).querySelector('app-test-screen > [role="status"]') as HTMLElement;

    it('reads a pause and injected input from a region already on the test screen', async () => {
      const fixture = await testing();
      await send(fixture, 'test:guide', VIEW);
      const status = region(fixture);
      expect(text(status)).toBe('');
      await send(fixture, 'test:event', key(E, false, 0, 1_000), key(E, true, 0, 51_000));
      const injected =
        'INJECTED INPUT. Events that did not come from a physical keyboard stay in the event ' +
        'list, marked injected, and are left out of every count.';
      expect(text(status)).toBe(injected);
      await send(fixture, 'test:event', PAUSED);
      expect(region(fixture)).toBe(status);
      expect(text(status)).toMatch(
        /^PAUSED · WINDOW LOST FOCUS \d{2}:\d{2}\. Click back into the window to continue\. No key was down at that moment\. This round of E will be repeated\.$/,
      );
      await send(fixture, 'test:event', RESUMED);
      expect(text(status)).toBe(injected);
      expect(all(fixture, '.notice[role], .footer__note[role]')).toEqual([]);
    });

    it('reads each new prompt, but not each press', async () => {
      const fixture = await testing();
      await send(fixture, 'test:guide', VIEW);
      const ask = el(fixture).querySelector('.prompt__ask') as HTMLElement;
      expect(ask.getAttribute('aria-live')).toBe('polite');
      expect(el(fixture).querySelector('.prompt [aria-live]:not(.prompt__ask)')).toBeNull();
      await send(fixture, 'test:guide', { ...VIEW, key: G, count: 0, index: 1 });
      expect(el(fixture).querySelector('.prompt__ask')).toBe(ask);
      expect(text(ask)).toBe('Press G ten times.');
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

    it('drops the second click of a double click on Pause, which lands on Continue', async () => {
      const fixture = await testing();
      await send(fixture, 'test:guide', VIEW);
      await nthClick(fixture, button(fixture, 'Pause'), 1);
      await send(fixture, 'test:event', PAUSED);
      await nthClick(fixture, button(fixture, 'Continue'), 2);
      expect(sent('continue_test')).toEqual([]);
      // Control: a click of its own continues, and a double click there doesn't pause again.
      await nthClick(fixture, button(fixture, 'Continue'), 1);
      expect(sent('continue_test')).toEqual([{}]);
      await send(fixture, 'test:event', RESUMED);
      await nthClick(fixture, button(fixture, 'Pause'), 2);
      expect(sent('pause_test')).toEqual([{}]);
    });

    it('reads a later pause without a click as a focus loss', async () => {
      const fixture = await testing();
      await send(fixture, 'test:guide', VIEW);
      await click(fixture, button(fixture, 'Pause'));
      await send(fixture, 'test:event', PAUSED, RESUMED, PAUSED);
      expect(textOf(fixture, '.notice__label')).toMatch(/^PAUSED · WINDOW LOST FOCUS \d{2}:\d{2}$/);
      expect(all(fixture, '.live__row--window')).toHaveLength(1);
    });

    it('skips the prompted key, naming the step on screen', async () => {
      const fixture = await testing();
      await send(fixture, 'test:guide', { ...VIEW, round: 1, index: 5 });
      await click(fixture, button(fixture, 'Skip this key'));
      expect(sent('skip_key')).toEqual([{ round: 1, index: 5 }]);
    });

    it('drops the second click of a double click, which would skip the next key', async () => {
      const fixture = await testing();
      await send(fixture, 'test:guide', VIEW);
      const skip = button(fixture, 'Skip this key');
      for (const detail of [1, 2]) {
        skip.dispatchEvent(new MouseEvent('click', { bubbles: true, detail }));
        await settle(fixture);
        // Rust draws the next step before the second click lands.
        if (detail === 1) await send(fixture, 'test:guide', { ...VIEW, index: 1 });
      }
      expect(sent('skip_key')).toEqual([{ round: 0, index: 0 }]);
    });

    it('drops the second click of a double click on Begin test, which lands on End test', async () => {
      const fixture = await render();
      await nthClick(fixture, button(fixture, 'Begin test'), 1);
      expect(textOf(fixture, '.steps__now')).toBe('02 Test');
      await nthClick(fixture, button(fixture, 'End test'), 2);
      expect(sent('end_test')).toEqual([]);
      expect(textOf(fixture, '.steps__now')).toBe('02 Test');
    });

    it('lets a click that began in a focus pause only bring the window back', async () => {
      const fixture = await testing();
      await send(fixture, 'test:guide', VIEW);
      await send(fixture, 'test:event', PAUSED);
      const skip = button(fixture, 'Skip this key');
      skip.dispatchEvent(new Event('pointerdown', { bubbles: true }));
      // The click brings the window back, and capture resumes before the click arrives.
      await send(fixture, 'test:event', RESUMED);
      await click(fixture, skip);
      expect(sent('skip_key')).toEqual([]);
      skip.dispatchEvent(new Event('pointerdown', { bubbles: true }));
      await click(fixture, skip);
      expect(sent('skip_key')).toEqual([{ round: 0, index: 0 }]);
    });

    it('skips during a pause the user asked for', async () => {
      const fixture = await testing();
      await send(fixture, 'test:guide', VIEW);
      await click(fixture, button(fixture, 'Pause'));
      await send(fixture, 'test:event', PAUSED);
      const skip = button(fixture, 'Skip this key');
      skip.dispatchEvent(new Event('pointerdown', { bubbles: true }));
      await click(fixture, skip);
      expect(sent('skip_key')).toEqual([{ round: 0, index: 0 }]);
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

  describe('while the test ends', () => {
    // end_test and skip_key each wait until the test answers them.
    function held(): { end: (result: TestResult) => void; skip: (why: string) => void } {
      const answer = { end: (_: TestResult) => {}, skip: (_: string) => {} };
      inApp((cmd) => {
        if (cmd === 'end_test') return new Promise<TestResult>((done) => (answer.end = done));
        if (cmd === 'skip_key') return new Promise<void>((_, fail) => (answer.skip = fail));
        return null;
      });
      return { end: (result) => answer.end(result), skip: (why) => answer.skip(why) };
    }

    it('offers no Pause, Continue or Skip this key until the findings arrive', async () => {
      const answer = held();
      const fixture = await testing();
      await send(fixture, 'test:guide', VIEW);
      await click(fixture, button(fixture, 'End test'));
      expect(
        ['Pause', 'Skip this key', 'End test'].map((b) => button(fixture, b).disabled),
      ).toEqual([true, true, true]);
      await send(fixture, 'test:event', PAUSED);
      expect(button(fixture, 'Continue').disabled).toBe(true);
      answer.end(EMPTY);
      await settle(fixture);
      expect(textOf(fixture, '.steps__now')).toBe('03 Findings');
    });

    it("keeps a command's failure that arrives after the test ended off the findings", async () => {
      const answer = held();
      const fixture = await testing();
      await send(fixture, 'test:guide', VIEW);
      await click(fixture, button(fixture, 'Skip this key'));
      await click(fixture, button(fixture, 'End test'));
      answer.end(EMPTY);
      await settle(fixture);
      answer.skip('No test is running.');
      await settle(fixture);
      expect(textOf(fixture, '.steps__now')).toBe('03 Findings');
      expect(textOf(fixture, '.footer__note')).toBe(
        'Reports hold per-key counts and timings only.',
      );
    });

    it("keeps a command's failure off the next test", async () => {
      const answer = held();
      const fixture = await testing();
      await send(fixture, 'test:guide', VIEW);
      await click(fixture, button(fixture, 'Skip this key'));
      await click(fixture, button(fixture, 'End test'));
      answer.end(EMPTY);
      await settle(fixture);
      await click(fixture, button(fixture, 'Test again'));
      answer.skip('No test is running.');
      await settle(fixture);
      expect(textOf(fixture, '.steps__now')).toBe('02 Test');
      expect(textOf(fixture, '.footer__note')).toBe('Capture stops when the window loses focus.');
    });
  });

  describe('the swap test', () => {
    const CAPTURE = 'Capture stops when the window loses focus.';
    const note = (fixture: ComponentFixture<App>) => textOf(fixture, '.footer__note');

    it('says it is the swap test in the footer, and only during the swap test', async () => {
      const fixture = await offered([FOLLOWS, EMPTY]);
      await click(fixture, button(fixture, 'Run the swap test'));
      await click(fixture, button(fixture, 'Switches swapped. Test both keys'));
      expect(note(fixture)).toBe(GOLDEN.result.swap?.note);
      expect(note(fixture)).toBe('Swap test. Both keys, 3 rounds.');
      await click(fixture, button(fixture, 'End test'));
      await click(fixture, button(fixture, 'New test'));
      await click(fixture, button(fixture, 'Begin test'));
      expect(textOf(fixture, '.steps__now')).toBe('02 Test');
      expect(note(fixture)).toBe(CAPTURE);
    });

    it("gives a command's reason in place of the swap note", async () => {
      const fixture = await offered([], (cmd) => {
        if (cmd === 'start_swap_test') return [1];
        if (cmd === 'skip_key') throw 'No test is running.';
        return null;
      });
      await click(fixture, button(fixture, 'Run the swap test'));
      await click(fixture, button(fixture, 'Switches swapped. Test both keys'));
      await send(fixture, 'test:guide', { ...VIEW, keys: 2, total: 180, tallies: [] });
      await click(fixture, button(fixture, 'Skip this key'));
      expect(note(fixture)).toBe('No test is running.');
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
