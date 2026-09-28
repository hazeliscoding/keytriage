import { type ComponentFixture } from '@angular/core/testing';
import { App } from './app';
import type { TestEvent } from './ipc';
import {
  all,
  button,
  cap,
  click,
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

function cells(fixture: ComponentFixture<App>): string[][] {
  return all(fixture, '.live__row').map((row) => [...row.children].map((cell) => text(cell)));
}

function state(fixture: ComponentFixture<App>, scan: number): string {
  return cap(fixture, scan).className;
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
      expect(sent('stop_test')).toHaveLength(1);
      expect(textOf(fixture, '.steps__now')).toBe('01 Keyboard');
      await send(fixture, 'test:event', key(E, false, OWN, 2_000));
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
});
