import { TestBed, type ComponentFixture } from '@angular/core/testing';
import { App } from './app';
import type { KeyboardGroup } from './ipc';
import { TestRun } from './test-run';
import {
  GOLDEN,
  all,
  button,
  cap,
  click,
  el,
  key,
  leaveApp,
  nthClick,
  offered,
  send,
  sent,
  settle,
  started,
  text,
  textOf,
} from './testing/harness';

const E = 0x12;
const G = 0x22;
const J = 0x24;
const OWN = 1;
const OFFER = GOLDEN.result.swap!;
const UNPLUGGED = "The keyboard isn't listed. Plug it back into the port it used, then try again.";

// Every cap drawn in `state`, in drawing order, with its tag and count.
function caps(fixture: ComponentFixture<App>, state: string): [number, string, string][] {
  return all(fixture, `.keyboard .cap--${state}`).map((c) => [
    Number(c.dataset['scan']),
    text(c.querySelector('.cap__tag')),
    text(c.querySelector('.cap__count')),
  ]);
}

// Lines whose parts sit side by side in spans, read with a space between the parts.
function parts(fixture: ComponentFixture<App>, selector: string): string[] {
  return all(fixture, selector).map((line) =>
    [...line.children].map((part) => text(part)).join(' '),
  );
}

const labels = (fixture: ComponentFixture<App>) => all(fixture, '.footer button').map(text);

async function instructed(
  answer?: (cmd: string, args: unknown) => unknown,
  groups?: KeyboardGroup[],
): Promise<ComponentFixture<App>> {
  const fixture = await offered([], answer, groups);
  await click(fixture, button(fixture, 'Run the swap test'));
  return fixture;
}

describe('Swap screen', () => {
  afterEach(leaveApp);

  describe('the instructions', () => {
    it("names the pair in the kicker and gives Rust's words verbatim", async () => {
      const fixture = await instructed();
      expect(textOf(fixture, '.findings__list > .kicker')).toBe('Swap test // E with G');
      expect(all(fixture, '.finding')).toHaveLength(1);
      expect(textOf(fixture, '.finding__title')).toBe(OFFER.title);
      expect(textOf(fixture, '.swap__about')).toBe(OFFER.knownGood);
      expect(parts(fixture, '.swap__steps .finding__line')).toEqual(
        OFFER.steps.map((step, i) => `0${i + 1} ${step}`),
      );
      expect(OFFER.steps).toHaveLength(4);
      expect(textOf(fixture, '.finding__next .kicker')).toBe('What the result means');
      expect(all(fixture, '.finding__next .finding__para').map(text)).toEqual([OFFER.means]);
    });

    it('marks the suspect A and the known-good key B, and the other tested keys without counts', async () => {
      const fixture = await instructed();
      expect(caps(fixture, 'flagged')).toEqual([[E, 'A', '']]);
      expect(caps(fixture, 'prompted')).toEqual([[G, 'B', '']]);
      expect(caps(fixture, 'counted')).toEqual([[J, '', '']]);
      expect(all(fixture, '.keyboard .cap__count')).toHaveLength(0);
      expect(all(fixture, '.legend > *').map(text)).toEqual([
        'Suspect switch',
        'Known-good switch',
      ]);
      expect(el(fixture).querySelector('.summary')).toBeNull();
    });

    it('stays at step 03 with capture off, and offers only Back and the retest', async () => {
      const fixture = await instructed();
      expect(textOf(fixture, '.steps__now')).toBe('03 Findings');
      expect(textOf(fixture, '.header__capture')).toBe('Capture off');
      expect(labels(fixture)).toEqual(['Back to findings', 'Switches swapped. Test both keys']);
      expect(button(fixture, 'Back to findings').classList).toContain('btn--ghost');
      expect(button(fixture, 'Switches swapped. Test both keys').classList).toContain(
        'btn--primary',
      );
      expect(textOf(fixture, '.footer__note')).toBe('');
    });
  });

  describe('back and start', () => {
    it('goes back to the findings as they were, and still exports the main report', async () => {
      const fixture = await instructed((cmd) =>
        cmd === 'export_report' ? 'keytriage-2026.09.28-1412.json' : null,
      );
      await click(fixture, button(fixture, 'Back to findings'));
      expect(textOf(fixture, '.steps__now')).toBe('03 Findings');
      expect(all(fixture, '.finding__title').map(text)).toEqual(['Possible chatter']);
      expect(labels(fixture)).toEqual(['New test', 'Export report', 'Run the swap test']);
      await click(fixture, button(fixture, 'Export report'));
      const args = sent('export_report');
      expect(args).toHaveLength(1);
      expect(Object.keys(args[0] as object)).toEqual(['name']);
      expect(textOf(fixture, '.footer__note')).toBe('Saved as keytriage-2026.09.28-1412.json.');
      expect(sent('start_swap_test')).toEqual([]);
    });

    it('drops the second click of a double click on Back to findings, which lands on New test', async () => {
      const fixture = await instructed();
      await nthClick(fixture, button(fixture, 'Back to findings'), 1);
      await nthClick(fixture, button(fixture, 'New test'), 2);
      expect(textOf(fixture, '.steps__now')).toBe('03 Findings');
      expect(all(fixture, '.finding__title').map(text)).toEqual(['Possible chatter']);
      expect(labels(fixture)).toEqual(['New test', 'Export report', 'Run the swap test']);
      expect(sent('list_keyboards')).toHaveLength(1);
      // Control: a click of its own goes to Start.
      await nthClick(fixture, button(fixture, 'New test'), 1);
      expect(textOf(fixture, '.steps__now')).toBe('01 Keyboard');
    });

    it('starts the swap test with no arguments, and shows it on the test screen with its note', async () => {
      const fixture = await instructed();
      await click(fixture, button(fixture, 'Switches swapped. Test both keys'));
      expect(sent('start_swap_test')).toEqual([{}]);
      expect(started()).toEqual([GOLDEN.plan]);
      expect(textOf(fixture, '.steps__now')).toBe('02 Test');
      expect(textOf(fixture, '.header__capture')).toBe('● Capturing');
      expect(textOf(fixture, '.footer__note')).toBe(OFFER.note);
      expect(OFFER.note).toBe('Swap test. Both keys, 3 rounds.');
    });

    it('keeps the instructions while the start is pending', async () => {
      let answer: (handles: number[]) => void = () => undefined;
      const fixture = await instructed((cmd) =>
        cmd === 'start_swap_test' ? new Promise<number[]>((done) => (answer = done)) : null,
      );
      await click(fixture, button(fixture, 'Switches swapped. Test both keys'));
      expect(textOf(fixture, '.findings__list > .kicker')).toBe('Swap test // E with G');
      expect(textOf(fixture, '.steps__now')).toBe('03 Findings');
      expect(button(fixture, 'Switches swapped. Test both keys').disabled).toBe(true);
      await click(fixture, button(fixture, 'Switches swapped. Test both keys'));
      answer([OWN]);
      await settle(fixture);
      expect(sent('start_swap_test')).toHaveLength(1);
      expect(textOf(fixture, '.steps__now')).toBe('02 Test');
    });

    it('drops the second click of a double click on Run the swap test, which lands on the retest', async () => {
      const fixture = await offered();
      await nthClick(fixture, button(fixture, 'Run the swap test'), 1);
      await nthClick(fixture, button(fixture, 'Switches swapped. Test both keys'), 2);
      expect(sent('start_swap_test')).toEqual([]);
      expect(textOf(fixture, '.findings__list > .kicker')).toBe('Swap test // E with G');
      expect(textOf(fixture, '.header__capture')).toBe('Capture off');
      // Control: a click of its own starts the retest.
      await nthClick(fixture, button(fixture, 'Switches swapped. Test both keys'), 1);
      expect(sent('start_swap_test')).toEqual([{}]);
    });
  });

  describe('failures and re-finding the keyboard', () => {
    it('keeps the instructions and the findings, with the reason, when the swap test cannot start', async () => {
      const fixture = await instructed((cmd) => {
        if (cmd === 'start_swap_test') throw UNPLUGGED;
        return null;
      });
      await click(fixture, button(fixture, 'Switches swapped. Test both keys'));
      expect(textOf(fixture, '.steps__now')).toBe('03 Findings');
      expect(textOf(fixture, '.header__capture')).toBe('Capture off');
      expect(textOf(fixture, '.findings__list > .kicker')).toBe('Swap test // E with G');
      expect(textOf(fixture, '.footer__note')).toBe(UNPLUGGED);
      expect(button(fixture, 'Switches swapped. Test both keys').disabled).toBe(false);
      expect(sent('list_keyboards')).toHaveLength(1);
      await click(fixture, button(fixture, 'Back to findings'));
      expect(all(fixture, '.finding__title').map(text)).toEqual(['Possible chatter']);
      expect(textOf(fixture, '.footer__note')).toBe(
        'Reports hold per-key counts and timings only.',
      );
    });

    it('lists the keyboards again and reads the new handle when Rust found the keyboard replugged', async () => {
      const groups: KeyboardGroup[] = structuredClone(GOLDEN.keyboards);
      const fixture = await instructed((cmd) => (cmd === 'start_swap_test' ? [41] : null), groups);
      // Windows lists the same keyboard on a new handle once it is back.
      groups[0].entries[0].handle = 41;
      await click(fixture, button(fixture, 'Switches swapped. Test both keys'));
      await fixture.whenStable();
      expect(sent('list_keyboards')).toHaveLength(2);
      expect(TestBed.inject(TestRun).picked()).toBe(41);
      expect(textOf(fixture, '.header__context')).toBe('Synthetic keyboard · 0000:0001');
      await send(fixture, 'test:event', key(E, false, 41, 1_000));
      expect(cap(fixture, E).className).toBe('cap cap--down');
      expect(all(fixture, '.live__row')[0].children[3].textContent?.trim()).toBe('0000:0001');
      // Control: the old handle no longer draws.
      await send(fixture, 'test:event', key(E, true, 41, 61_000), key(G, false, OWN, 90_000));
      expect(cap(fixture, E).className).toBe('cap');
      expect(cap(fixture, G).className).toBe('cap');
    });

    it('lists nothing again when the keyboard kept its handles', async () => {
      const fixture = await instructed();
      await click(fixture, button(fixture, 'Switches swapped. Test both keys'));
      expect(sent('list_keyboards')).toHaveLength(1);
      await send(fixture, 'test:event', key(E, false, OWN, 1_000));
      expect(cap(fixture, E).className).toBe('cap cap--down');
    });
  });
});
