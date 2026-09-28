import { TestBed, type ComponentFixture } from '@angular/core/testing';
import { App } from './app';
import type { OutcomeView, TestResult } from './ipc';
import { TestRun } from './test-run';
import {
  FOLLOWS,
  all,
  button,
  calls,
  click,
  el,
  leaveApp,
  offered,
  sent,
  settle,
  text,
  textOf,
} from './testing/harness';

const E = 0x12;
const G = 0x22;
const E_CLEAR =
  'E: no extra key-downs in 90 presses, so a rate above 3.4% would very likely have shown';
const G_CLEAR =
  'G: no extra key-downs in 90 presses, so a rate above 3.4% would very likely have shown';

function retest(outcome: OutcomeView, keys: [number, number][]): TestResult {
  return { ...FOLLOWS, keys: keys.map(([scan, count]) => ({ scan, count })), outcome };
}

// Each outcome as the engine words it for the golden run's pair, E with G.
const STAYS = retest(
  {
    outcome: 'stays',
    tile: E,
    flagged: [E],
    title: 'The fault stayed on E.',
    confidence: 'very-high',
    level: 'Very high',
    strong: true,
    evidence: ['E: 18 of 90 presses sent an extra key-down (a rate of at least 13%)', G_CLEAR],
    diagnosis:
      'A known-good switch in the E socket shows the same fault. The socket, the solder joints ' +
      'under it, or the matrix trace is the most likely cause. The original switch is probably ' +
      'fine.',
    next: [
      'Inspect the E socket for a loose pin and the joints under it for a crack. Reflowing the ' +
        'socket pins is a small job. If the board is under warranty, this result is what the ' +
        'vendor needs.',
    ],
  },
  [
    [E, 108],
    [G, 90],
  ],
);

const BOTH = retest(
  {
    outcome: 'both',
    tile: E,
    flagged: [E, G],
    title: 'The fault showed on both keys.',
    confidence: 'medium',
    level: 'Medium',
    strong: false,
    evidence: [
      'E: 18 of 90 presses sent an extra key-down (a rate of at least 13%)',
      'G: 4 of 90 presses sent an extra key-down (a rate of at least 1.7%)',
    ],
    diagnosis:
      "The fault showed on E with the known-good switch and on G with the E switch, so this swap can't " +
      'tell the switch from the socket. That points past a single switch, to firmware debounce or ' +
      'the keyboard as a whole.',
    next: [
      "If the keyboard's firmware lets you, raise its debounce time to 10 ms, then 15 ms, and test " +
        'again.',
    ],
  },
  [
    [E, 108],
    [G, 94],
  ],
);

const GONE = retest(
  {
    outcome: 'gone',
    tile: null,
    flagged: [],
    title: 'Neither key showed the fault.',
    confidence: 'low',
    level: 'Low',
    strong: false,
    evidence: [E_CLEAR, G_CLEAR],
    diagnosis:
      'Both keys registered normally after the swap. Reseating the switches may have cleared a ' +
      "poor contact, or the fault comes and goes and didn't show in this test.",
    next: [
      'Use the keyboard for a day. If the fault returns on E, repeat this swap test. If it returns ' +
        'on G, the switch that came from E is the likely cause.',
    ],
  },
  [
    [E, 90],
    [G, 90],
  ],
);

const UNCLEAR = retest(
  {
    outcome: 'unclear',
    tile: null,
    flagged: [],
    title: "This swap test can't place the fault.",
    confidence: null,
    level: null,
    strong: false,
    evidence: [
      "E: no extra key-downs in 20 presses, too few to rule out the first test's rate of at least 9.5%",
      'G: 1 of 30 presses sent an extra key-down, one short of a finding',
    ],
    diagnosis:
      'This swap test gave too little evidence on E and G to compare with the first test, so the ' +
      "first test's finding still stands.",
    next: [
      'Start a new test with the switches where they are now, and press each key as it is prompted.',
    ],
  },
  [
    [E, 20],
    [G, 31],
  ],
);

// Ends the main test, runs the swap test and ends it with `result`.
async function judged(
  result: TestResult,
  answer?: (cmd: string, args: unknown) => unknown,
): Promise<ComponentFixture<App>> {
  const fixture = await offered([result], answer);
  await click(fixture, button(fixture, 'Run the swap test'));
  await click(fixture, button(fixture, 'Switches swapped. Test both keys'));
  await click(fixture, button(fixture, 'End test'));
  return fixture;
}

function section(card: Element, title: string): Element {
  const found = [...card.querySelectorAll('.finding__section')].find(
    (s) => text(s.querySelector('.kicker')) === title,
  );
  if (!found) throw new Error(`no section "${title}"`);
  return found;
}

// Lines whose parts sit side by side in spans, read with a space between the parts.
function lines(node: Element, selector: string): string[] {
  return [...node.querySelectorAll(selector)].map((line) =>
    [...line.children].map((part) => text(part)).join(' '),
  );
}

function paras(node: Element): string[] {
  return [...node.querySelectorAll('.finding__para')].map((para) => text(para));
}

function caps(fixture: ComponentFixture<App>, state: string): [number, string, string][] {
  return all(fixture, `.keyboard .cap--${state}`).map((c) => [
    Number(c.dataset['scan']),
    text(c.querySelector('.cap__tag')),
    text(c.querySelector('.cap__count')),
  ]);
}

function summary(fixture: ComponentFixture<App>): string[] {
  return all(fixture, '.summary__item').map((item) =>
    [...item.children].map((part) => text(part)).join(': '),
  );
}

describe('Swap result screen', () => {
  afterEach(() => {
    leaveApp();
    vi.useRealTimers();
  });

  describe('the card', () => {
    const cases: [string, TestResult, string, string, string | null, boolean][] = [
      ['follows', FOLLOWS, 'finding__tile', 'G', 'Very high confidence', true],
      ['stays', STAYS, 'finding__tile', 'E', 'Very high confidence', true],
      ['both', BOTH, 'finding__tile', 'E', 'Medium confidence', false],
      ['gone', GONE, 'finding__tile finding__tile--clean', '—', 'Low confidence', false],
      ['unclear', UNCLEAR, 'finding__tile finding__tile--none', '—', null, false],
    ];

    it.each(cases)(
      "draws %s with its tile, badge and Rust's words",
      async (_, result, tile, label, badge, ink) => {
        const fixture = await judged(result);
        const outcome = result.outcome!;
        expect(textOf(fixture, '.steps__now')).toBe('03 Findings');
        expect(textOf(fixture, '.header__capture')).toBe('Capture off');
        expect(textOf(fixture, '.findings__list > .kicker')).toBe('Swap test // Result');
        const cards = all(fixture, '.finding');
        expect(cards).toHaveLength(1);
        const [card] = cards;
        expect(card.querySelector('.finding__tile')?.className).toBe(tile);
        expect(text(card.querySelector('.finding__tile'))).toBe(label);
        expect(card.querySelector('.finding__num')).toBeNull();
        expect(text(card.querySelector('.finding__title'))).toBe(outcome.title);
        const shown = card.querySelector('.badge');
        expect(shown ? text(shown) : null).toBe(badge);
        expect(shown?.classList.contains('badge--ink') ?? false).toBe(ink);
        expect(lines(section(card, 'Evidence'), '.finding__line')).toEqual(
          outcome.evidence.map((line) => `— ${line}`),
        );
        expect(paras(section(card, 'Updated diagnosis'))).toEqual([outcome.diagnosis]);
        expect(paras(section(card, 'Next step'))).toEqual(outcome.next);
        expect(section(card, 'Next step').classList).toContain('finding__next');
        expect(all(fixture, '.finding .kicker').map(text)).toEqual([
          'Evidence',
          'Updated diagnosis',
          'Next step',
        ]);
      },
    );

    it("tells a changed diagnosis from the engine's words", async () => {
      const changed = { ...FOLLOWS.outcome!, diagnosis: 'The switch is the cause.' };
      const fixture = await judged({ ...FOLLOWS, outcome: changed });
      expect(textOf(fixture, '.finding__para')).toBe('The switch is the cause.');
      expect(textOf(fixture, '.finding__para')).not.toBe(FOLLOWS.outcome!.diagnosis);
    });
  });

  describe('the keyboard and the summary', () => {
    it("draws the retest's counts and tags the key that showed the fault with the card's number", async () => {
      const fixture = await judged(FOLLOWS);
      expect(caps(fixture, 'flagged')).toEqual([[G, '01', '108']]);
      expect(caps(fixture, 'counted')).toEqual([[E, '', '90']]);
      expect(all(fixture, '.keyboard .cap--flagged, .keyboard .cap--counted')).toHaveLength(2);
      expect(all(fixture, '.legend > *').map(text)).toEqual([
        'Fault seen after the swap',
        'Tested, no finding',
      ]);
    });

    it('tags both keys when both showed the fault', async () => {
      const fixture = await judged(BOTH);
      expect(caps(fixture, 'flagged')).toEqual([
        [E, '01', '108'],
        [G, '01', '94'],
      ]);
    });

    it('drops the flagged legend line when no key showed the fault', async () => {
      const fixture = await judged(GONE);
      expect(caps(fixture, 'flagged')).toEqual([]);
      expect(caps(fixture, 'counted')).toEqual([
        [E, '', '90'],
        [G, '', '90'],
      ]);
      expect(all(fixture, '.legend > *').map(text)).toEqual(['Tested, no finding']);
    });

    it('sums up the main test, as the findings did', async () => {
      const fixture = await offered([FOLLOWS]);
      const before = summary(fixture);
      await click(fixture, button(fixture, 'Run the swap test'));
      await click(fixture, button(fixture, 'Switches swapped. Test both keys'));
      await click(fixture, button(fixture, 'End test'));
      expect(summary(fixture)).toEqual(before);
      expect(before.slice(0, 4)).toEqual([
        'Keyboard: Synthetic keyboard · 0000:0001',
        'Layout: 75% ANSI',
        'Board: Hot-swap',
        'Rounds: 3 × 10 presses',
      ]);
    });
  });

  describe('the footer', () => {
    it('offers New test and Export report, with Export report primary', async () => {
      const fixture = await judged(FOLLOWS);
      expect(all(fixture, '.footer button').map(text)).toEqual(['New test', 'Export report']);
      expect(button(fixture, 'Export report').classList).toContain('btn--primary');
      expect(button(fixture, 'New test').classList).toContain('btn--ghost');
      expect(textOf(fixture, '.footer__note')).toBe(
        'Reports hold per-key counts and timings only.',
      );
    });

    it("exports once, by the swap test's start, and names the saved file", async () => {
      vi.useFakeTimers({ toFake: ['Date'] });
      vi.setSystemTime(new Date(2026, 8, 28, 14, 2));
      let answer: (name: string | null) => void = () => undefined;
      const fixture = await offered([FOLLOWS], (cmd) => {
        if (cmd === 'start_swap_test') return [1];
        if (cmd === 'export_report') return new Promise((done) => (answer = done));
        return null;
      });
      await click(fixture, button(fixture, 'Run the swap test'));
      vi.setSystemTime(new Date(2026, 8, 28, 14, 12));
      await click(fixture, button(fixture, 'Switches swapped. Test both keys'));
      await click(fixture, button(fixture, 'End test'));
      expect(summary(fixture)[4]).toBe('Started: 2026.09.28 14:02');
      const exportButton = button(fixture, 'Export report');
      await click(fixture, exportButton);
      expect(exportButton.disabled).toBe(true);
      await click(fixture, exportButton);
      expect(sent('export_report')).toEqual([{ name: 'keytriage-2026.09.28-1412.json' }]);
      answer('keytriage-2026.09.28-1412.json');
      await settle(fixture);
      expect(exportButton.disabled).toBe(false);
      expect(textOf(fixture, '.footer__note')).toBe('Saved as keytriage-2026.09.28-1412.json.');
      expect(textOf(fixture, '[role="status"]')).toBe('Saved as keytriage-2026.09.28-1412.json.');
    });

    it('changes nothing when the dialog is cancelled', async () => {
      const fixture = await judged(FOLLOWS, (cmd) => (cmd === 'start_swap_test' ? [1] : null));
      await click(fixture, button(fixture, 'Export report'));
      expect(sent('export_report')).toHaveLength(1);
      expect(textOf(fixture, '.footer__note')).toBe(
        'Reports hold per-key counts and timings only.',
      );
      expect(textOf(fixture, '.steps__now')).toBe('03 Findings');
      expect(button(fixture, 'Export report').disabled).toBe(false);
    });

    it('gives the reason when the report cannot be saved', async () => {
      const fixture = await judged(FOLLOWS, (cmd) => {
        if (cmd === 'start_swap_test') return [1];
        if (cmd === 'export_report') throw 'The file could not be written.';
        return null;
      });
      await click(fixture, button(fixture, 'Export report'));
      expect(textOf(fixture, '.footer__note')).toBe('The file could not be written.');
    });

    it('goes back to Start for a new test, lists the keyboards again and forgets both tests', async () => {
      const fixture = await judged(FOLLOWS);
      const listed = () => calls().filter(([cmd]) => cmd === 'list_keyboards').length;
      const before = listed();
      await click(fixture, button(fixture, 'New test'));
      expect(textOf(fixture, '.steps__now')).toBe('01 Keyboard');
      expect(listed()).toBe(before + 1);
      expect(el(fixture).querySelector('.finding')).toBeNull();
      const run = TestBed.inject(TestRun);
      expect([run.findings(), run.swapped(), run.swapping()]).toEqual([null, null, false]);
    });
  });
});
