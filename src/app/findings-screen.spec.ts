import { type ComponentFixture } from '@angular/core/testing';
import { App } from './app';
import {
  RECONNECTED,
  type FindingView,
  type GuideView,
  type KeyName,
  type TestResult,
} from './ipc';
import { labelsFor, layout } from './layout';
import { PLAN } from './plan';
import {
  EMPTY,
  GOLDEN,
  after,
  all,
  button,
  cap,
  calls,
  click,
  el,
  inApp,
  key,
  leaveApp,
  send,
  sent,
  settle,
  started,
  testing,
  text,
  textOf,
} from './testing/harness';

const E = 0x12;
const G = 0x22;
const K = 0x25;
const OWN = 11;

const CHATTER: FindingView = {
  key: E,
  kind: 'chatter',
  confidence: 'very-high',
  title: 'Possible chatter',
  level: 'Very high',
  strong: true,
  evidence: [
    '6 of 30 presses sent an extra key-down (a rate of at least 8.9%)',
    'The extra key-downs came 5 ms after a release',
    'Reproduced in 3 of 3 rounds',
  ],
  causes: ['Switch contacts', 'Hot-swap socket or solder joint', 'Firmware debounce'],
  next: [
    'Swap the E switch with the G switch and test both keys again.',
    "If the keyboard's firmware lets you, raise its debounce time to 10 ms, then 15 ms, and test again.",
  ],
  gaps: [
    { label: '<4', count: 0 },
    { label: '4–12', count: 6 },
    { label: '12–20', count: 0 },
    { label: '20–36', count: 0 },
    { label: '36–100', count: 3 },
    { label: '100+', count: 24 },
  ],
};

const DEAD: FindingView = {
  key: K,
  kind: 'dead',
  confidence: 'medium',
  title: 'Possible dead key',
  level: 'Medium',
  strong: false,
  evidence: ['No key-down in 1 of 3 rounds (10 presses asked)'],
  causes: ['Switch contacts'],
  next: ['Test K again: 3 rounds of 10 presses, with rounds of another key in between.'],
  gaps: null,
};

const FOUND: TestResult = {
  rules: 4,
  findings: [CHATTER, DEAD],
  notes: ['K had been down for 0.4 s when the test paused.'],
  clean: ['G: no extra key-downs in 30 presses.'],
  keys: [
    { scan: E, count: 36 },
    { scan: G, count: 30 },
    { scan: K, count: 0 },
  ],
  resolution: 'This keyboard showed no 8 or 16 ms reporting schedule in this test.',
  swap: null,
  outcome: null,
};

const CLEAN: TestResult = {
  ...EMPTY,
  notes: ['Nothing arrived from this keyboard during the test.'],
  clean: [
    'E: no extra key-downs in 30 presses. A rate above 9.5% would very likely have shown.',
    'G: no extra key-downs in 30 presses. A rate above 9.5% would very likely have shown.',
  ],
  keys: [
    { scan: E, count: 30 },
    { scan: G, count: 30 },
  ],
  resolution: 'This keyboard showed no 8 or 16 ms reporting schedule in this test.',
};

// The golden run's last view. Once the plan is done, round and index stay on its last step.
const DONE: GuideView = {
  key: null,
  asked: 10,
  count: 0,
  round: 2,
  rounds: 3,
  index: 2,
  keys: 3,
  done: 90,
  total: 90,
  tallies: [
    [E, 35],
    [G, 30],
    [0x24, 30],
  ],
  waitMs: 150,
};

const GOLDEN_PLAN = {
  provide: PLAN,
  useValue: { keys: () => [G, 0x24, E], rounds: 3, presses: 10 },
};

// A view mid-plan: E is prompted in round 1, after G and J closed theirs.
const MID: GuideView = {
  key: E,
  asked: 10,
  count: 4,
  round: 0,
  rounds: 3,
  index: 2,
  keys: 3,
  done: 24,
  total: 90,
  tallies: [
    [G, 10],
    [0x24, 10],
    [E, 4],
  ],
};

// Ends a test at the view `last`, if Rust sent one, and shows what `result` holds.
async function finished(result: TestResult, last?: GuideView): Promise<ComponentFixture<App>> {
  inApp((cmd) => (cmd === 'end_test' ? result : null));
  const fixture = await testing([GOLDEN_PLAN]);
  if (last) await send(fixture, 'test:guide', last);
  await click(fixture, button(fixture, 'End test'));
  return fixture;
}

function lines(node: Element, selector: string): string[] {
  return [...node.querySelectorAll(selector)].map((line) => text(line));
}

// Lines whose parts sit side by side in spans, read with a space between the parts.
function parts(node: Element, selector: string): string[] {
  return [...node.querySelectorAll(selector)].map((line) =>
    [...line.children].map((part) => text(part)).join(' '),
  );
}

function section(card: Element, title: string): Element {
  const found = [...card.querySelectorAll('.finding__section, .hist')].find(
    (s) => text(s.querySelector('.kicker')) === title,
  );
  if (!found) throw new Error(`no section "${title}"`);
  return found;
}

describe('Findings screen', () => {
  afterEach(leaveApp);

  describe('findings', () => {
    it('numbers the cards in the order Rust sends them, with their words', async () => {
      const fixture = await finished(FOUND);
      expect(textOf(fixture, '.steps__now')).toBe('03 Findings');
      expect(lines(el(fixture), '.findings__head > *')).toEqual([
        'Findings · 2',
        'Ordered by confidence',
      ]);
      const cards = all(fixture, '.finding');
      expect(cards.map((c) => text(c.querySelector('.finding__num')))).toEqual(['01', '02']);
      expect(cards.map((c) => text(c.querySelector('.finding__key')))).toEqual(['E', 'K']);
      expect(cards.map((c) => text(c.querySelector('.finding__title')))).toEqual([
        'Possible chatter',
        'Possible dead key',
      ]);
      const badges = cards.map((c) => c.querySelector('.badge') as HTMLElement);
      expect(badges.map(text)).toEqual(['Very high confidence', 'Medium confidence']);
      expect(badges.map((b) => b.classList.contains('badge--ink'))).toEqual([true, false]);

      const [chatter, dead] = cards;
      expect(parts(section(chatter, 'Evidence'), '.finding__line')).toEqual(
        CHATTER.evidence.map((line) => `— ${line}`),
      );
      expect(parts(section(chatter, 'Other possible causes'), '.finding__line')).toEqual([
        '01 Switch contacts',
        '02 Hot-swap socket or solder joint',
        '03 Firmware debounce',
      ]);
      expect(lines(section(chatter, 'Next test'), '.finding__para')).toEqual(CHATTER.next);
      expect(lines(dead, '.finding__para')).toEqual(DEAD.next);
    });

    it("draws the chatter card's release gaps in 6 bars, and none on other cards", async () => {
      const fixture = await finished(FOUND);
      const [chatter, dead] = all(fixture, '.finding');
      const hist = section(chatter, 'Release to the next key-down, ms');
      const heights = [...hist.querySelectorAll<HTMLElement>('.hist__bar')].map(
        (bar) => bar.style.height,
      );
      expect(heights).toEqual(['0%', '25%', '0%', '0%', '13%', '100%']);
      expect(parts(hist, '.hist__label')).toEqual([
        '<4 0',
        '4–12 6',
        '12–20 0',
        '20–36 0',
        '36–100 3',
        '100+ 24',
      ]);
      expect(dead.querySelector('.hist')).toBeNull();
    });

    it('flags each finding on the keyboard and counts the other tested keys', async () => {
      const fixture = await finished(FOUND);
      const at = (scan: number) => cap(fixture, scan);
      expect(at(E).className).toBe('cap cap--flagged');
      expect(text(at(E).querySelector('.cap__tag'))).toBe('01');
      expect(text(at(E).querySelector('.cap__count'))).toBe('36');
      expect(at(K).className).toBe('cap cap--flagged');
      expect(text(at(K).querySelector('.cap__tag'))).toBe('02');
      expect(at(K).querySelector('.cap__count')).toBeNull();
      expect(at(G).className).toBe('cap cap--counted');
      expect(text(at(G).querySelector('.cap__count'))).toBe('30');
      expect(at(0x11).className).toBe('cap');
      expect(lines(el(fixture), '.legend > *')).toEqual([
        'Flagged, numbered by finding',
        'Tested, no finding',
        'Counts are key-downs registered over all rounds.',
      ]);
    });

    it("tags a key named by two findings with the first one's number", async () => {
      const stuck: FindingView = { ...DEAD, key: E, kind: 'stuck', title: 'Possible stuck key' };
      const fixture = await finished({ ...FOUND, findings: [CHATTER, stuck] });
      expect(all(fixture, '.cap--flagged')).toHaveLength(1);
      expect(text(cap(fixture, E).querySelector('.cap__tag'))).toBe('01');
      expect(all(fixture, '.finding__key').map(text)).toEqual(['E', 'E']);
    });

    it('sums up the test and gives its limits with the notes', async () => {
      const fixture = await finished(FOUND);
      const summary = all(fixture, '.summary__item').map((item) => [
        text(item.children[0]),
        text(item.children[1]),
      ]);
      expect(summary.slice(0, 4)).toEqual([
        ['Keyboard', 'HID Keyboard Device · 046D:C52B'],
        ['Layout', '75% ANSI'],
        ['Board', 'Hot-swap'],
        ['Rounds', '3 × 10 presses'],
      ]);
      expect(summary[4]).toEqual([
        'Started',
        expect.stringMatching(/^\d{4}\.\d{2}\.\d{2} \d{2}:\d{2}$/),
      ]);
      expect(summary[5]).toEqual(['Duration', '00:00']);
      expect(lines(el(fixture), '.limits > *')).toEqual([
        'Limits',
        'The test sees what the firmware reports after its own debounce. This keyboard showed ' +
          'no 8 or 16 ms reporting schedule in this test. A finding describes evidence and ' +
          'likelihood, not a verdict on a part.',
        'K had been down for 0.4 s when the test paused.',
      ]);
    });
  });

  describe('a clean result', () => {
    it('says no fault was found, with the plan, the clean lines and the limits', async () => {
      const fixture = await finished(CLEAN, DONE);
      expect(lines(el(fixture), '.findings__head > *')).toEqual(['Findings · 0']);
      const [card] = all(fixture, '.finding');
      expect(card.querySelector('.finding__tile')?.classList).toContain('finding__tile--clean');
      expect(text(card.querySelector('.finding__title'))).toBe('No faults found in this test.');
      expect(text(card.querySelector('.badge--clean'))).toBe('Clean');
      expect(parts(section(card, 'Evidence'), '.finding__line')).toEqual([
        '— 3 keys, 10 presses in each of 3 rounds',
        ...CLEAN.clean.map((line) => `— ${line}`),
      ]);
      expect(parts(section(card, 'What this does not rule out'), '.finding__line')).toEqual([
        '01 Faults that appear only after warm-up or under sustained use',
        "02 Chatter shorter than the firmware's own debounce window",
        "03 Keys outside this test's set",
      ]);
      expect(lines(section(card, 'Next test'), '.finding__para')).toEqual([
        'If the fault is intermittent, run the test again while it is happening. Type for ten ' +
          'minutes first, then test again.',
      ]);
      expect(all(fixture, '.cap--flagged')).toHaveLength(0);
      expect(cap(fixture, E).className).toBe('cap cap--counted');
      expect(lines(el(fixture), '.legend > *')).toEqual([
        'Tested, no finding',
        'Counts are key-downs registered over all rounds.',
      ]);
      expect(lines(el(fixture), '.limits > *').slice(1)).toEqual([
        expect.stringContaining(
          'This keyboard showed no 8 or 16 ms reporting schedule in this test.',
        ),
        'Nothing arrived from this keyboard during the test.',
      ]);
    });

    it('says how far a test that ended early got, in place of the plan', async () => {
      const fixture = await finished(CLEAN, MID);
      const [card] = all(fixture, '.finding');
      expect(text(card.querySelector('.badge--clean'))).toBe('Clean');
      expect(parts(section(card, 'Evidence'), '.finding__line')).toEqual([
        '— Ended after 24 of 90 presses, 2 of 3 keys',
        ...CLEAN.clean.map((line) => `— ${line}`),
      ]);
      const rounds = all(fixture, '.summary__item').find(
        (item) => text(item.children[0]) === 'Rounds',
      );
      expect(text(rounds?.children[1])).toBe('3 × 10 presses');
    });

    it('says nothing was tested, with no Clean badge, when no key kept a round', async () => {
      const fixture = await finished(EMPTY, {
        ...MID,
        key: G,
        count: 0,
        index: 0,
        done: 0,
        tallies: [],
      });
      const [card] = all(fixture, '.finding');
      expect(all(fixture, '.finding')).toHaveLength(1);
      expect(text(card.querySelector('.finding__title'))).toBe('Nothing was tested.');
      expect(card.querySelector('.badge')).toBeNull();
      expect(card.querySelector('.finding__tile--clean')).toBeNull();
      expect(card.querySelector('.finding__tile--none')).not.toBeNull();
      expect(parts(section(card, 'Evidence'), '.finding__line')).toEqual([
        '— Ended after 0 of 90 presses, 0 of 3 keys',
      ]);
      expect(lines(section(card, 'Next test'), '.finding__para')).toEqual([
        'Test again, and press each key as it is prompted.',
      ]);
      expect(lines(card, '.kicker')).toEqual(['Evidence', 'Next test']);
    });
  });

  describe('ending', () => {
    it("ends by itself after the wait Rust sends with the plan's last view, naming the drawn keys", async () => {
      inApp((cmd) => (cmd === 'end_test' ? FOUND : null));
      const fixture = await testing([GOLDEN_PLAN]);
      await send(fixture, 'test:event', key(E, false, OWN, 1_000), key(E, true, OWN, 61_000));
      await send(fixture, 'test:guide', DONE);
      await settle(fixture);
      // The last press's chatter may still be on its way, so capture stays on for the wait.
      expect(sent('end_test')).toHaveLength(0);
      expect(textOf(fixture, '.steps__now')).toBe('02 Test');
      await after(fixture, DONE.waitMs);
      const labels = (sent('end_test') as { labels: KeyName[] }[]).map((args) => args.labels);
      expect(labels).toEqual([labelsFor(layout('75%', 'ANSI'))]);
      expect(labels[0]).toContainEqual({ scan: 18, name: 'E' });
      expect(sent('stop_test')).toHaveLength(0);
      expect(textOf(fixture, '.steps__now')).toBe('03 Findings');
      expect(textOf(fixture, '.header__capture')).toBe('Capture off');
      expect(all(fixture, '.live__row')).toHaveLength(0);
    });

    it('ends once when the wait runs out and End test cross', async () => {
      let answer: (result: TestResult) => void = () => undefined;
      inApp((cmd) =>
        cmd === 'end_test' ? new Promise<TestResult>((done) => (answer = done)) : null,
      );
      const fixture = await testing([GOLDEN_PLAN]);
      await send(fixture, 'test:guide', DONE);
      await after(fixture, DONE.waitMs);
      expect(button(fixture, 'End test').disabled).toBe(true);
      await click(fixture, button(fixture, 'End test'));
      answer(FOUND);
      await settle(fixture);
      expect(sent('end_test')).toHaveLength(1);
      expect(all(fixture, '.finding')).toHaveLength(2);
    });

    it('ends at once when End test is clicked during the wait, and only once', async () => {
      let answer: (result: TestResult) => void = () => undefined;
      inApp((cmd) =>
        cmd === 'end_test' ? new Promise<TestResult>((done) => (answer = done)) : null,
      );
      const fixture = await testing([GOLDEN_PLAN]);
      await send(fixture, 'test:guide', DONE);
      await click(fixture, button(fixture, 'End test'));
      expect(sent('end_test')).toHaveLength(1);
      await after(fixture, DONE.waitMs);
      answer(FOUND);
      await settle(fixture);
      expect(sent('end_test')).toHaveLength(1);
      expect(all(fixture, '.finding')).toHaveLength(2);
    });

    it('stays on the test with the reason when the test cannot end', async () => {
      inApp((cmd) => {
        if (cmd === 'end_test') throw 'This test has no plan to diagnose.';
        return null;
      });
      const fixture = await testing([GOLDEN_PLAN]);
      await click(fixture, button(fixture, 'End test'));
      expect(textOf(fixture, '.steps__now')).toBe('02 Test');
      expect(textOf(fixture, '.footer__note')).toBe('This test has no plan to diagnose.');
      expect(button(fixture, 'End test').disabled).toBe(false);
    });
  });

  describe('the footer', () => {
    it('offers the swap test as the primary button when Rust sends an offer', async () => {
      const offer = GOLDEN.result.swap;
      const fixture = await finished({ ...FOUND, swap: offer });
      expect(all(fixture, '.footer button').map(text)).toEqual([
        'New test',
        'Export report',
        'Run the swap test',
      ]);
      expect(button(fixture, 'Run the swap test').classList).toContain('btn--primary');
      await click(fixture, button(fixture, 'Run the swap test'));
      expect(textOf(fixture, '.findings__list > .kicker')).toBe('Swap test // E with G');
      expect(started()).toHaveLength(1);
    });

    // Rust sends no offer for a soldered or laptop board, or without a known-good key.
    it('offers Test again, and no swap test, when Rust sends no offer', async () => {
      const fixture = await finished(FOUND);
      expect(all(fixture, '.footer button').map(text)).toEqual([
        'New test',
        'Export report',
        'Test again',
      ]);
      expect(all(fixture, 'button').some((b) => text(b) === 'Run the swap test')).toBe(false);
    });

    it('tests again with the same keyboard, plan and board', async () => {
      const fixture = await finished(FOUND);
      expect(all(fixture, '.footer button').map(text)).toEqual([
        'New test',
        'Export report',
        'Test again',
      ]);
      expect(textOf(fixture, '.footer__note')).toBe(
        'Reports hold per-key counts and timings only.',
      );
      expect(button(fixture, 'Test again').classList).toContain('btn--primary');
      await click(fixture, button(fixture, 'Test again'));
      const [first, again] = started();
      expect(again).toEqual(first);
      expect(again.keys).toEqual([G, 0x24, E]);
      expect(textOf(fixture, '.steps__now')).toBe('02 Test');
    });

    it('keeps the findings when a new test cannot start', async () => {
      const fixture = await finished(FOUND);
      inApp((cmd) => {
        if (cmd === 'start_test') throw 'Capture could not start.';
        return null;
      });
      await click(fixture, button(fixture, 'Test again'));
      expect(textOf(fixture, '.steps__now')).toBe('03 Findings');
      expect(all(fixture, '.finding')).toHaveLength(2);
      expect(textOf(fixture, '.footer__note')).toBe('Capture could not start.');
    });

    it('goes back to Start, which lists the keyboards again, when the keyboard was reconnected', async () => {
      const fixture = await finished(FOUND);
      inApp((cmd) => {
        if (cmd === 'start_test') throw RECONNECTED;
        return null;
      });
      await click(fixture, button(fixture, 'Test again'));
      expect(textOf(fixture, '.steps__now')).toBe('01 Keyboard');
      expect(textOf(fixture, '.footer__note')).toBe(RECONNECTED);
      expect(sent('list_keyboards')).toHaveLength(1);
    });

    it('goes back to Start for a new test and lists the keyboards again', async () => {
      const fixture = await finished(FOUND);
      const listed = () => calls().filter(([cmd]) => cmd === 'list_keyboards').length;
      const before = listed();
      await click(fixture, button(fixture, 'New test'));
      expect(textOf(fixture, '.steps__now')).toBe('01 Keyboard');
      expect(listed()).toBe(before + 1);
      expect(all(fixture, '.finding')).toHaveLength(0);
    });
  });

  describe('export', () => {
    // Ends a test. Each export_report call then waits until the test answers it.
    async function exporting(): Promise<{
      fixture: ComponentFixture<App>;
      answer: (reply: string | null | Error) => Promise<void>;
    }> {
      let pending: { done: (name: string | null) => void; fail: (why: string) => void }[] = [];
      inApp((cmd) => {
        if (cmd === 'end_test') return FOUND;
        if (cmd === 'export_report') {
          return new Promise<string | null>((done, fail) => pending.push({ done, fail }));
        }
        return null;
      });
      const fixture = await testing([GOLDEN_PLAN]);
      await click(fixture, button(fixture, 'End test'));
      const answer = async (reply: string | null | Error) => {
        const [call] = pending;
        pending = pending.slice(1);
        if (reply instanceof Error) call.fail(reply.message);
        else call.done(reply);
        await settle(fixture);
      };
      return { fixture, answer };
    }

    const note = (fixture: ComponentFixture<App>) => textOf(fixture, '.footer__note');
    const status = (fixture: ComponentFixture<App>) => textOf(fixture, '[role="status"]');

    it("sends only the file name of the test's start, once, and waits for the dialog", async () => {
      const { fixture, answer } = await exporting();
      const exportButton = button(fixture, 'Export report');
      await click(fixture, exportButton);
      expect(exportButton.disabled).toBe(true);
      await click(fixture, exportButton);
      const args = sent('export_report');
      expect(args).toHaveLength(1);
      expect(Object.keys(args[0] as object)).toEqual(['name']);
      const { name } = args[0] as { name: string };
      expect(name).toMatch(/^keytriage-\d{4}\.\d{2}\.\d{2}-\d{4}\.json$/);
      const started = all(fixture, '.summary__item')
        .map((item) => text(item.children[1]))
        .find((value) => /^\d{4}\./.test(value));
      expect(name).toBe(`keytriage-${started?.replace(' ', '-').replace(':', '')}.json`);

      await answer('keytriage-2026.09.28-1412.json');
      expect(exportButton.disabled).toBe(false);
      expect(status(fixture)).toBe('Saved as keytriage-2026.09.28-1412.json.');
    });

    it('changes nothing when the dialog is cancelled', async () => {
      const { fixture, answer } = await exporting();
      await click(fixture, button(fixture, 'Export report'));
      await answer(null);
      expect(note(fixture)).toBe('Reports hold per-key counts and timings only.');
      expect(status(fixture)).toBe('');
      await click(fixture, button(fixture, 'Export report'));
      await answer('keytriage-2026.09.28-1412.json');
      await click(fixture, button(fixture, 'Export report'));
      await answer(null);
      expect(note(fixture)).toBe('Saved as keytriage-2026.09.28-1412.json.');
      expect(button(fixture, 'Export report').disabled).toBe(false);
    });

    it('gives the reason when the report cannot be saved', async () => {
      const { fixture, answer } = await exporting();
      await click(fixture, button(fixture, 'Export report'));
      await answer(new Error('The file could not be written.'));
      expect(status(fixture)).toBe('The file could not be written.');
      expect(textOf(fixture, '.steps__now')).toBe('03 Findings');
      expect(button(fixture, 'Export report').disabled).toBe(false);
    });
  });
});
