import { type ComponentFixture } from '@angular/core/testing';
import { App } from './app';
import {
  GOLDEN,
  SAVED,
  SWAP,
  all,
  button,
  calls,
  click,
  leaveApp,
  sent,
  swapRun,
  text,
  textOf,
  timesSent,
  type SwapName,
} from './testing/harness';

// The Done-when runs through the page: the golden chatter run, then the swap test Rust offers with
// its findings, replayed as Rust emits it when the fault moved with the switch and when it stayed.
// Rust judges both and words the result. golden::m4_done_swap_follows_and_stays pins that judgment
// and walks the retest's exported file with export::no_sequence.

const E = 0x12;
const G = 0x22;
const OFFER = GOLDEN.result.swap!;

// Every cap drawn in `state`, in drawing order, with its tag and count.
function caps(fixture: ComponentFixture<App>, state: string): [number, string, string][] {
  return all(fixture, `.keyboard .cap--${state}`).map((c) => [
    Number(c.dataset['scan']),
    text(c.querySelector('.cap__tag')),
    text(c.querySelector('.cap__count')),
  ]);
}

// Lines whose parts sit side by side in spans, read with a space between the parts.
function parts(node: Element, selector: string): string[] {
  return [...node.querySelectorAll(selector)].map((line) =>
    [...line.children].map((part) => text(part)).join(' '),
  );
}

function paras(node: Element): string[] {
  return [...node.querySelectorAll('.finding__para')].map((para) => text(para));
}

function section(card: Element, title: string): Element {
  const found = [...card.querySelectorAll('.finding__section')].find(
    (s) => text(s.querySelector('.kicker')) === title,
  );
  if (!found) throw new Error(`no section "${title}"`);
  return found;
}

// The swap instructions as the page shows them.
function instructions(fixture: ComponentFixture<App>) {
  const page = fixture.nativeElement as HTMLElement;
  return {
    kicker: textOf(fixture, '.findings__list > .kicker'),
    title: textOf(fixture, '.finding__title'),
    knownGood: textOf(fixture, '.swap__about'),
    steps: parts(page, '.swap__steps .finding__line'),
    means: paras(page.querySelector('.finding__next') ?? page),
    marks: [...caps(fixture, 'flagged'), ...caps(fixture, 'prompted')],
  };
}

// The test screen at the retest's first view.
function prompt(fixture: ComponentFixture<App>) {
  const [step] = all(fixture, '.progress__line');
  return {
    now: textOf(fixture, '.steps__now'),
    step: [...step.children].map((cell) => text(cell)).join(' · '),
    ask: textOf(fixture, '.prompt__ask'),
    count: textOf(fixture, '.prompt__count'),
    note: textOf(fixture, '.footer__note'),
  };
}

// Runs the golden test and the named swap run, reading the instructions and the retest's first
// view on the way.
async function judged(name: SwapName) {
  const seen: {
    instructed?: ReturnType<typeof instructions>;
    prompted?: ReturnType<typeof prompt>;
  } = {};
  const fixture = await swapRun(name, {
    look: (page, screen) => {
      if (screen === 'swap') seen.instructed = instructions(page);
      else seen.prompted = prompt(page);
    },
  });
  return { fixture, ...seen };
}

const CASES: [string, SwapName, SwapName, number, number][] = [
  ['follows the switch', 'follows', 'stays', G, E],
  ['stays', 'stays', 'follows', E, G],
];

describe('the swap test after the synthetic chatter run', () => {
  afterEach(leaveApp);

  it.each(CASES)(
    'the fault that %s reaches the result screen as Rust judged it',
    async (_, name, other, flagged, clean) => {
      const { fixture, instructed, prompted } = await judged(name);
      const { result } = SWAP.runs[name];
      const outcome = result.outcome!;

      expect(instructed).toEqual({
        kicker: 'Swap test // E with G',
        title: OFFER.title,
        knownGood: OFFER.knownGood,
        steps: OFFER.steps.map((step, i) => `0${i + 1} ${step}`),
        means: [OFFER.means],
        marks: [
          [E, 'A', ''],
          [G, 'B', ''],
        ],
      });
      expect(prompted).toEqual({
        now: '02 Test',
        step: 'Round 1 of 3 · Key 1 of 2',
        ask: 'Press E 30 times.',
        count: '0 / 30',
        note: OFFER.note,
      });
      expect(OFFER.note).toBe('Swap test. Both keys, 3 rounds.');

      expect(textOf(fixture, '.steps__now')).toBe('03 Findings');
      expect(textOf(fixture, '.header__capture')).toBe('Capture off');
      expect(textOf(fixture, '.findings__list > .kicker')).toBe('Swap test // Result');
      const cards = all(fixture, '.finding');
      expect(cards).toHaveLength(1);
      const [card] = cards;
      const tile = card.querySelector('.finding__tile');
      expect(tile?.className).toBe('finding__tile');
      expect(text(tile)).toBe(name === 'follows' ? 'G' : 'E');
      expect(text(card.querySelector('.finding__title'))).toBe(outcome.title);
      const badge = card.querySelector('.badge');
      expect(text(badge)).toBe('Very high confidence');
      expect(badge?.classList).toContain('badge--ink');
      expect(parts(section(card, 'Evidence'), '.finding__line')).toEqual(
        outcome.evidence.map((line) => `— ${line}`),
      );
      expect(paras(section(card, 'Updated diagnosis'))).toEqual([outcome.diagnosis]);
      expect(paras(section(card, 'Next step'))).toEqual(outcome.next);
      // Control: the other run's judgment is not what the page shows.
      expect(text(card)).not.toContain(SWAP.runs[other].result.outcome!.title);

      // The drawing has the retest's counts, and the summary the main test's.
      expect(caps(fixture, 'flagged')).toEqual([[flagged, '01', '108']]);
      expect(caps(fixture, 'counted')).toEqual([[clean, '', '90']]);
      expect(result.keys).toContainEqual({ scan: flagged, count: 108 });
      const summary = all(fixture, '.summary__item').map((item) =>
        [...item.children].map((part) => text(part)).join(': '),
      );
      expect(summary.slice(0, 4)).toEqual([
        'Keyboard: Synthetic keyboard · 0000:0001',
        'Layout: 75% ANSI',
        'Board: Hot-swap',
        'Rounds: 3 × 10 presses',
      ]);

      await click(fixture, button(fixture, 'Export report'));
      expect(textOf(fixture, '[role="status"]')).toBe(`Saved as ${SAVED}.`);
      expect(calls().map(([cmd]) => cmd)).toEqual([
        'list_keyboards',
        'start_test',
        'end_test',
        'start_swap_test',
        'end_test',
        'export_report',
      ]);
      expect(sent('start_swap_test')).toEqual([{}]);
      expect(sent('export_report')).toEqual([
        { name: expect.stringMatching(/^keytriage-\d{4}\.\d{2}\.\d{2}-\d{4}\.json$/) },
      ]);
      // Rust writes the file from the retest's own Report::saved().
      expect(timesSent(calls(), [GOLDEN.script, SWAP.runs[name].script])).toEqual([]);
    },
  );

  describe('positive controls', () => {
    it("tells a changed diagnosis from the engine's words", async () => {
      const { result } = SWAP.runs.follows;
      const outcome = result.outcome!;
      const changed = 'The E switch now sits in the G socket, and the fault is gone from E.';
      const fixture = await swapRun('follows', {
        result: { ...result, outcome: { ...outcome, diagnosis: changed } },
      });
      const [card] = all(fixture, '.finding');
      expect(paras(section(card, 'Updated diagnosis'))).toEqual([changed]);
      expect(text(card)).not.toContain(outcome.diagnosis);
    });

    it("finds a swap test's event time in a command", () => {
      const { script } = SWAP.runs.stays;
      const micros = script
        .flatMap((step) => (step.event === 'test:event' ? [step.payload.micros] : []))
        .at(-1);
      expect(timesSent([['export_report', { name: SAVED, at: micros }]], [script])).toEqual([
        micros,
      ]);
      // The main run never reached that time, so only the swap test's script finds it.
      expect(timesSent([['export_report', { name: SAVED, at: micros }]], [GOLDEN.script])).toEqual(
        [],
      );
    });
  });
});
