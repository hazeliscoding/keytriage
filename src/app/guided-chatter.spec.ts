import { type ComponentFixture } from '@angular/core/testing';
import { emit } from '@tauri-apps/api/event';
import { App } from './app';
import type { KeyName, TestResult } from './ipc';
import {
  GOLDEN,
  GOLDEN_PLAN,
  after,
  all,
  button,
  calls,
  cap,
  click,
  inApp,
  leaveApp,
  render,
  sent,
  settle,
  started,
  text,
  textOf,
  timesSent,
  type Emitted,
} from './testing/harness';

// The Done-when run through the page: the engine's synthetic chatter stream on E, as Rust emits it
// and as end_test words it. The exported file never reaches the page, so Rust's tests cover it.

const E = 0x12;
const G = 0x22;
const J = 0x24;
const R = 0x13;
const SAVED = 'keytriage-2026.09.28-1412.json';

const VIEWS = GOLDEN.script.flatMap((step) => (step.event === 'test:guide' ? [step.payload] : []));
const PROMPTS = VIEWS.filter((view) => view.key !== null);
const END = GOLDEN.script.findIndex(
  (step) => step.event === 'test:guide' && step.payload.key === null,
);
const LAST = GOLDEN.script[END];
const WAIT = VIEWS.at(-1)?.waitMs;

// What the test screen showed at one of Rust's views.
interface Shown {
  ask: string;
  count: string;
  step: string;
  presses: string;
  newest: string[];
  prompted: number[];
  // The counts drawn on E, G and J.
  caps: string[];
}

function cells(node: Element | undefined): string[] {
  return node ? [...node.children].map((cell) => text(cell)) : [];
}

function shown(fixture: ComponentFixture<App>): Shown {
  const [step, readout] = all(fixture, '.progress__line').map(cells);
  return {
    ask: textOf(fixture, '.prompt__ask'),
    count: textOf(fixture, '.prompt__count'),
    step: step.join(' · '),
    presses: readout[1],
    newest: cells(all(fixture, '.live__row')[0]),
    prompted: caps(fixture, 'prompted').map(([scan]) => scan),
    caps: [E, G, J].map((scan) => text(cap(fixture, scan).querySelector('.cap__count'))),
  };
}

interface Run {
  fixture: ComponentFixture<App>;
  // The page at every view that prompts a key, in order, when the run was read.
  views: Shown[];
  // The script step whose delivery sent end_test, or the script's length when it came after the
  // whole script.
  endedAt: number;
}

// Begins the golden test on its keyboard, 75% ANSI and the preselected board, delivers `script` in
// order and lets end_test answer with `result`. Reading the page at every view renders it 185
// times, so only the run that checks the views asks for it.
async function run(
  script: readonly Emitted[],
  { result = GOLDEN.result, read = false }: { result?: TestResult; read?: boolean } = {},
): Promise<Run> {
  inApp((cmd) => {
    if (cmd === 'end_test') return result;
    if (cmd === 'export_report') return SAVED;
    return null;
  }, GOLDEN.keyboards);
  const fixture = await render([GOLDEN_PLAN]);
  await click(fixture, button(fixture, 'Begin test'));
  const views: Shown[] = [];
  let endedAt = -1;
  for (const [i, step] of script.entries()) {
    await emit(step.event, step.payload);
    if (endedAt < 0 && sent('end_test').length) endedAt = i;
    if (read && step.event === 'test:guide' && step.payload.key !== null) {
      await fixture.whenStable();
      views.push(shown(fixture));
    }
  }
  // The page ends the test once the wait Rust sends with the last view is over.
  await after(fixture, WAIT);
  if (endedAt < 0 && sent('end_test').length) endedAt = script.length;
  return { fixture, views, endedAt };
}

// A run straight to Rust's last view, for the controls that need only the findings.
const finished = async (result: TestResult) => (await run([LAST], { result })).fixture;

// Consecutive views on the same step of the plan.
function steps(views: Shown[]): Shown[][] {
  const out: Shown[][] = [];
  for (const view of views) {
    const last = out[out.length - 1];
    if (last?.[0].step === view.step) last.push(view);
    else out.push([view]);
  }
  return out;
}

function lines(section: Element | undefined, selector: string): string[] {
  return [...(section?.querySelectorAll(selector) ?? [])].map((line) =>
    [...line.children].map((part) => text(part)).join(' '),
  );
}

// A card's evidence, causes and next steps as printed, without their dashes and numbers.
function words(card: Element): { evidence: string[]; causes: string[]; next: string[] } {
  const [evidence, causes, next] = [...card.querySelectorAll('.finding__section')];
  const bare = (line: string) => line.replace(/^(—|\d{2}) /, '');
  return {
    evidence: lines(evidence, '.finding__line').map(bare),
    causes: lines(causes, '.finding__line').map(bare),
    next: [...(next?.querySelectorAll('.finding__para') ?? [])].map((p) => text(p)),
  };
}

// Every cap drawn in `state`, in drawing order, with its tag and count.
function caps(fixture: ComponentFixture<App>, state: string): [number, string, string][] {
  return all(fixture, `.keyboard .cap--${state}`).map((c) => [
    Number(c.dataset['scan']),
    text(c.querySelector('.cap__tag')),
    text(c.querySelector('.cap__count')),
  ]);
}

describe('the synthetic chatter run', () => {
  afterEach(leaveApp);

  it('prompts G, J and E in turn and shows the counts Rust sends', async () => {
    const { views } = await run(GOLDEN.script, { read: true });
    expect(started()).toEqual([GOLDEN.plan]);

    expect(views[0]).toEqual({
      ask: 'Press G ten times.',
      count: '0 / 10',
      step: 'Round 1 of 3 · Key 1 of 3',
      presses: '0 of 90 presses',
      newest: [],
      prompted: [G],
      caps: ['', '', ''],
    });
    expect(views[1].newest).toEqual(['G', 'down', '600.0', '0000:0001']);
    expect(views.every((v, i) => i === 0 || v.newest[3] === '0000:0001')).toBe(true);

    // A press counts at its release, and the 10th release closes the round, so each step's last
    // view reads 9.
    const asks = ['Press G ten times.', 'Press J ten times.', 'Press E ten times.'];
    const scans = [G, J, E];
    expect(
      steps(views).map((s) => [s[0].step, s[0].ask, s[0].prompted, s[0].count, s.at(-1)?.count]),
    ).toEqual(
      [1, 2, 3].flatMap((round) =>
        asks.map((ask, i) => [
          `Round ${round} of 3 · Key ${i + 1} of 3`,
          ask,
          [scans[i]],
          '0 / 10',
          '9 / 10',
        ]),
      ),
    );
    // The page counts nothing itself: every view reads as Rust sent it.
    expect(views.map((v) => v.count)).toEqual(PROMPTS.map((v) => `${v.count} / ${v.asked}`));
    expect(views.map((v) => v.presses)).toEqual(PROMPTS.map((v) => `${v.done} of 90 presses`));

    // Each key-down moves E's cap and each counted release the count. The 5th answer's 5 ms
    // key-down is drawn on the cap and left out of the presses.
    const firstE = views.filter((v) => v.step === 'Round 1 of 3 · Key 3 of 3');
    expect(firstE.map((v) => v.count)).toEqual(
      [0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9].map((n) => `${n} / 10`),
    );
    expect(firstE.map((v) => v.caps[0])).toEqual(
      ['', 1, 1, 2, 2, 3, 3, 4, 4, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11].map(String),
    );

    expect(views.at(-1)).toEqual({
      ask: 'Press E ten times.',
      count: '9 / 10',
      step: 'Round 3 of 3 · Key 3 of 3',
      presses: '89 of 90 presses',
      newest: ['E', 'down', '32 726.9', '0000:0001'],
      prompted: [],
      caps: ['35', '30', '30'],
    });
  });

  it("ends after Rust's last view and wait, and shows the chatter on E in the engine's words", async () => {
    const { fixture, endedAt } = await run(GOLDEN.script);
    // E's last chatter fragment follows the last view, and the test still takes it.
    expect(END).toBe(GOLDEN.script.length - 3);
    expect(WAIT).toBe(150);
    expect(endedAt).toBe(GOLDEN.script.length);
    const labels = (sent('end_test') as { labels: KeyName[] }[]).map((args) => args.labels);
    expect(labels).toHaveLength(1);
    expect(labels[0]).toEqual(expect.arrayContaining(GOLDEN.labels));
    expect(sent('stop_test')).toEqual([]);

    expect(textOf(fixture, '.steps__now')).toBe('03 Findings');
    expect(textOf(fixture, '.header__capture')).toBe('Capture off');
    expect(all(fixture, '.live__row')).toHaveLength(0);

    const [finding] = GOLDEN.result.findings;
    const cards = all(fixture, '.finding');
    expect(cards).toHaveLength(1);
    const [card] = cards;
    expect(text(card.querySelector('.finding__num'))).toBe('01');
    expect(text(card.querySelector('.finding__key'))).toBe('E');
    expect(text(card.querySelector('.finding__title'))).toBe('Possible chatter');
    expect(text(card.querySelector('.badge'))).toBe('Very high confidence');
    expect(words(card)).toEqual({
      evidence: finding.evidence,
      causes: finding.causes,
      next: finding.next,
    });
    expect(words(card).evidence[0]).toBe(
      '6 of 30 presses sent an extra key-down (a rate of at least 9.5%)',
    );
    expect(lines(card.querySelector('.hist') ?? undefined, '.hist__label')).toEqual(
      (finding.gaps ?? []).map((bar) => `${bar.label} ${bar.count}`),
    );

    expect(caps(fixture, 'flagged')).toEqual([[E, '01', '36']]);
    expect(caps(fixture, 'counted')).toEqual([
      [G, '', '30'],
      [J, '', '30'],
    ]);

    const summary = all(fixture, '.summary__item').map((item) => cells(item).join(': '));
    expect(summary.slice(0, 4)).toEqual([
      'Keyboard: Synthetic keyboard · 0000:0001',
      'Layout: 75% ANSI',
      'Board: Hot-swap',
      'Rounds: 3 × 10 presses',
    ]);
    expect(textOf(fixture, '.limits')).toContain(GOLDEN.result.resolution);
  });

  it('exports by name only, and no event of the test leaves the page', async () => {
    const { fixture } = await run(GOLDEN.script);
    await click(fixture, button(fixture, 'Export report'));
    expect(sent('export_report')).toEqual([
      { name: expect.stringMatching(/^keytriage-\d{4}\.\d{2}\.\d{2}-\d{4}\.json$/) },
    ]);
    expect(textOf(fixture, '[role="status"]')).toBe(`Saved as ${SAVED}.`);
    expect(calls().map(([cmd]) => cmd)).toEqual([
      'list_keyboards',
      'start_test',
      'end_test',
      'export_report',
    ]);
    // Rust writes the file from its own Report::saved(). golden::m3_done_guided_chatter walks this
    // run's file with export::no_sequence, and export::ns01 shows it catching an event list.
    expect(timesSent(calls(), [GOLDEN.script])).toEqual([]);
  });

  describe('positive controls', () => {
    it('reads a second flagged cap', async () => {
      const [chatter] = GOLDEN.result.findings;
      const fixture = await finished({
        ...GOLDEN.result,
        findings: [chatter, { ...chatter, key: R }],
      });
      expect(caps(fixture, 'flagged')).toEqual([
        [E, '01', '36'],
        [R, '02', ''],
      ]);
    });

    it("tells a changed line from the engine's words", async () => {
      const [chatter] = GOLDEN.result.findings;
      const changed = ['5 of 30 presses sent an extra key-down', ...chatter.evidence.slice(1)];
      const fixture = await finished({
        ...GOLDEN.result,
        findings: [{ ...chatter, evidence: changed }],
      });
      const [card] = all(fixture, '.finding');
      expect(words(card).evidence).toEqual(changed);
      expect(words(card).evidence).not.toEqual(chatter.evidence);
    });

    it('finds an event time in a command', () => {
      const [, first] = GOLDEN.script;
      expect(
        timesSent([['export_report', { name: SAVED, events: [first.payload] }]], [GOLDEN.script]),
      ).toEqual([600_000]);
    });
  });
});
