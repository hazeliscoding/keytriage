import { DestroyRef, Injectable, computed, inject, signal } from '@angular/core';
import { Bridge, reasonOf } from './bridge';
import { clock, fmtMicros } from './format';
import {
  RECONNECTED,
  type Board,
  type Events,
  type GuideView,
  type KeyboardGroup,
  type PlanArgs,
  type TestEvent,
  type TestResult,
} from './ipc';
import { cancelKeys, dropFocus } from './keys';
import { capLabel, labelsFor, layout, type Layout, type Size, type Std } from './layout';
import { PLAN } from './plan';

export type Screen = 'start' | 'starting' | 'test' | 'findings';

export interface BoardOption {
  id: Board;
  name: string;
  about: string;
}

export const BOARDS: readonly BoardOption[] = [
  { id: 'hot-swap', name: 'Hot-swap', about: 'Switches pull out. The swap test is available.' },
  {
    id: 'soldered',
    name: 'Soldered',
    about: 'Switches are fixed to the PCB. Next steps skip the swap.',
  },
  {
    id: 'laptop',
    name: 'Laptop or scissor-switch',
    about: 'Built in. Next steps cover the keycap, the scissor and the membrane.',
  },
];

// A line of the live list. A window line marks a focus loss.
export interface LiveRow {
  seq: number;
  key: string;
  edge: string;
  t: string;
  device: string;
  kind: 'key' | 'injected' | 'window';
}

export interface Pause {
  reason: 'user' | 'focus';
  at: string;
}

// A finished test as the findings screen shows it. It holds Rust's words and per-key counts, never
// an event or a time within the test.
export interface Findings {
  result: TestResult;
  keyboard: string;
  layout: Layout;
  board: string;
  keys: number;
  rounds: number;
  presses: number;
  started: Date;
  // Whole seconds, pauses left out.
  duration: number;
}

const KEPT_ROWS = 60;

// The page's state. Screens are a signal rather than routes, so nothing adds a history entry.
@Injectable({ providedIn: 'root' })
export class TestRun {
  private readonly bridge = inject(Bridge);
  private readonly plan = inject(PLAN);
  private listing = 0;

  readonly screen = signal<Screen>('start');
  // Null until the first list arrives, so the picker doesn't claim there is no keyboard too early.
  readonly groups = signal<KeyboardGroup[] | null>(null);
  // The entry the user clicked. Any entry picks its whole group, and handles are unique across groups.
  readonly picked = signal<number | null>(null);
  readonly std = signal<Std>('ANSI');
  readonly size = signal<Size>('75%');
  readonly board = signal<Board>('hot-swap');
  // The footer's reason when a command fails.
  readonly note = signal('');

  readonly group = computed(() => {
    const picked = this.picked();
    return this.groups()?.find((g) => g.entries.some((e) => e.handle === picked)) ?? null;
  });
  readonly layout = computed(() => layout(this.size(), this.std()));
  readonly boardName = computed(() => BOARDS.find((b) => b.id === this.board())?.name ?? '');

  // The live list, newest first. It is the only ordered record on the page, so it stays in memory
  // and is cleared when the test ends.
  readonly rows = signal<readonly LiveRow[]>([]);
  // Scan codes down on the tested keyboard. Other keyboards and injected input never change them.
  readonly held = signal<ReadonlySet<number>>(new Set());
  readonly interrupted = signal<readonly number[]>([]);
  readonly injected = signal(0);
  readonly pause = signal<Pause | null>(null);
  // Rust's view of the prompt, kept as sent: the page never counts presses itself.
  readonly guide = signal<GuideView | null>(null);
  readonly startedAt = signal<Date | null>(null);
  // True while end_test runs, so the last view and a click on End test end the test once.
  readonly ending = signal(false);
  readonly findings = signal<Findings | null>(null);

  // A test started from the findings keeps them on screen until it runs, or until it fails and
  // they come back.
  private readonly origin = signal<Screen>('start');
  readonly shown = computed(() => (this.screen() === 'starting' ? this.origin() : this.screen()));

  // The page's own clock, with pauses left out: `spent` ms before the stretch that began `since`.
  private readonly timer = signal<{ spent: number; since: number | null }>({
    spent: 0,
    since: null,
  });
  private readonly now = signal(0);
  // Whole seconds of the test so far.
  readonly elapsed = computed(() => {
    const { spent, since } = this.timer();
    return Math.floor((spent + (since === null ? 0 : Math.max(0, this.now() - since))) / 1000);
  });

  private handles: ReadonlySet<number> = new Set();
  private asked: PlanArgs | null = null;
  private seq = 0;
  private ticking: ReturnType<typeof setInterval> | undefined;
  private closing: ReturnType<typeof setTimeout> | undefined;
  private uncancel: (() => void) | null = null;
  // Set between a click on Pause and the pause it causes, which then reads as the user's own.
  private pauseAsked = false;

  constructor() {
    this.listen('test:event', (event) => this.fold(event));
    this.listen('test:guide', (view) => {
      this.guide.set(view);
      // Rust sends no key once the plan is done.
      if (view.key === null) this.endAfter(view.waitMs);
    });
    inject(DestroyRef).onDestroy(() => this.halt());
  }

  async listKeyboards(): Promise<void> {
    const call = ++this.listing;
    let groups: KeyboardGroup[] = [];
    try {
      groups = await this.bridge.listKeyboards();
    } catch (error) {
      if (call === this.listing) this.note.set(reasonOf(error));
    }
    if (call !== this.listing) return;
    this.groups.set(groups);
    const picked = this.picked();
    if (!groups.some((g) => g.entries.some((e) => e.handle === picked))) {
      this.picked.set(groups[0]?.entries[0]?.handle ?? null);
    }
  }

  async begin(): Promise<void> {
    const group = this.group();
    const screen = this.screen();
    if (!group || screen === 'starting' || screen === 'test') return;
    const plan: PlanArgs = {
      keyboard: group.entries.map((e) => e.handle),
      keys: this.plan.keys(this.layout()),
      rounds: this.plan.rounds,
      presses: this.plan.presses,
      board: this.board(),
    };
    this.note.set('');
    this.clear();
    this.handles = new Set(plan.keyboard);
    this.asked = plan;
    this.startedAt.set(new Date());
    this.origin.set(screen);
    this.run();
    this.screen.set('starting');
    try {
      await this.bridge.startTest(plan);
    } catch (error) {
      if (this.screen() !== 'starting') return;
      const reason = reasonOf(error);
      if (reason === RECONNECTED) {
        // The keyboard's handles are gone, so it is picked again from a new list. Start lists the
        // keyboards when it opens, and it may already be open.
        this.findings.set(null);
        this.leave('start');
        this.note.set(reason);
        if (screen === 'start') void this.listKeyboards();
        return;
      }
      this.leave(screen);
      this.note.set(reason);
      return;
    }
    if (this.screen() !== 'starting') return;
    this.findings.set(null);
    this.screen.set('test');
    // The plan's last view may have come before start_test returned.
    const last = this.guide();
    if (last?.key === null) this.endAfter(last.waitMs);
  }

  // A failure leaves the test running, with the reason in the footer.
  async end(): Promise<void> {
    if (this.screen() !== 'test' || this.ending()) return;
    this.ending.set(true);
    this.note.set('');
    const duration = Math.floor(this.spent() / 1000);
    const drawn = this.layout();
    let result: TestResult;
    try {
      result = await this.bridge.endTest(labelsFor(drawn));
    } catch (error) {
      this.note.set(reasonOf(error));
      return;
    } finally {
      this.ending.set(false);
    }
    if (this.screen() !== 'test') return;
    const group = this.group();
    const plan = this.asked;
    this.findings.set({
      result,
      keyboard: `${group?.name ?? '—'} · ${group?.id ?? '—'}`,
      layout: drawn,
      board: this.boardName(),
      keys: plan?.keys.length ?? 0,
      rounds: plan?.rounds ?? 0,
      presses: plan?.presses ?? 0,
      started: this.startedAt() ?? new Date(),
      duration,
    });
    this.leave('findings');
  }

  // Back to Start, which lists the keyboards again.
  newTest(): void {
    if (this.screen() !== 'findings') return;
    this.findings.set(null);
    this.leave('start');
  }

  async pauseTest(): Promise<void> {
    if (this.screen() !== 'test' || this.pause()) return;
    this.pauseAsked = true;
    if (!(await this.attempt(this.bridge.pauseTest()))) this.pauseAsked = false;
  }

  async continueTest(): Promise<void> {
    if (this.screen() === 'test') await this.attempt(this.bridge.continueTest());
  }

  // Rust skips only the step named, so a click sent as that round closed skips nothing. The second
  // click of a double click comes after the next step is drawn, so it is dropped here.
  async skipKey(clicks = 1): Promise<void> {
    const view = this.guide();
    if (this.screen() !== 'test' || clicks > 1 || !view || view.key === null) return;
    await this.attempt(this.bridge.skipKey(view.round, view.index));
  }

  // The last press's chatter can still be on its way when the plan is done, so the test ends once
  // Rust's wait is over. End test during the wait ends it at once.
  private endAfter(ms = 0): void {
    clearTimeout(this.closing);
    this.closing = setTimeout(() => void this.end(), ms);
  }

  // Runs a command during the test. A failure leaves the test as it was and shows its reason.
  private async attempt(command: Promise<void>): Promise<boolean> {
    this.note.set('');
    try {
      await command;
      return true;
    } catch (error) {
      this.note.set(reasonOf(error));
      return false;
    }
  }

  // The page listens for its whole life and takes events only during its own test, so the debug
  // echo's test never reaches the screen.
  private listen<K extends keyof Events>(event: K, take: (payload: Events[K]) => void): void {
    this.bridge
      .on(event, (payload) => {
        if (this.running()) take(payload);
      })
      .catch((error: unknown) => this.note.set(reasonOf(error)));
  }

  private running(): boolean {
    const screen = this.screen();
    return screen === 'starting' || screen === 'test';
  }

  private run(): void {
    dropFocus();
    this.uncancel ??= cancelKeys();
    this.ticking ??= setInterval(() => this.now.set(Date.now()), 1000);
    this.resumeClock();
  }

  private halt(): void {
    this.uncancel?.();
    this.uncancel = null;
    clearInterval(this.ticking);
    this.ticking = undefined;
    clearTimeout(this.closing);
    this.closing = undefined;
    this.pauseClock();
  }

  private leave(screen: Screen): void {
    this.halt();
    this.clear();
    this.note.set('');
    this.screen.set(screen);
  }

  private clear(): void {
    this.rows.set([]);
    this.held.set(new Set());
    this.interrupted.set([]);
    this.injected.set(0);
    this.pause.set(null);
    this.pauseAsked = false;
    this.guide.set(null);
    this.timer.set({ spent: 0, since: null });
  }

  private fold(event: TestEvent): void {
    switch (event.kind) {
      case 'key':
        return this.foldKey(event.scan, event.up, event.device, event.micros);
      case 'paused':
        return this.foldPause(event.interrupted);
      case 'resumed':
        return this.foldResume();
    }
  }

  private foldKey(scan: number, up: boolean, device: number, micros: number): void {
    const injected = device === 0;
    this.push({
      key: capLabel(this.layout(), scan),
      edge: up ? 'up' : 'down',
      t: fmtMicros(micros),
      device: this.deviceId(device),
      kind: injected ? 'injected' : 'key',
    });
    if (injected) {
      this.injected.update((n) => n + 1);
      return;
    }
    if (!this.handles.has(device)) return;
    const held = new Set(this.held());
    if (up) held.delete(scan);
    else held.add(scan);
    if (held.size !== this.held().size) this.held.set(held);
  }

  private foldPause(interrupted: readonly { device: number; scan: number }[]): void {
    const at = clock(new Date());
    const reason = this.pauseAsked ? 'user' : 'focus';
    this.pauseAsked = false;
    if (reason === 'focus') {
      this.push({ key: '—', edge: 'focus lost', t: at, device: 'window', kind: 'window' });
    }
    this.pause.set({ reason, at });
    const scans = interrupted.filter((k) => this.handles.has(k.device)).map((k) => k.scan);
    this.interrupted.set([...new Set(scans)]);
    // A key down at the pause is interrupted now, and its release may never arrive.
    this.held.set(new Set());
    this.pauseClock();
  }

  private foldResume(): void {
    this.pause.set(null);
    this.pauseAsked = false;
    this.interrupted.set([]);
    this.resumeClock();
  }

  private push(row: Omit<LiveRow, 'seq'>): void {
    const next = { seq: this.seq++, ...row };
    this.rows.update((rows) => [next, ...rows.slice(0, KEPT_ROWS - 1)]);
  }

  private deviceId(device: number): string {
    if (device === 0) return 'injected';
    const group = this.groups()?.find((g) => g.entries.some((e) => e.handle === device));
    return group?.id ?? '—';
  }

  // The clock to the millisecond. `elapsed` moves only on the interval's tick.
  private spent(): number {
    const { spent, since } = this.timer();
    return spent + (since === null ? 0 : Math.max(0, Date.now() - since));
  }

  private pauseClock(): void {
    const { spent, since } = this.timer();
    if (since === null) return;
    const now = Date.now();
    this.timer.set({ spent: spent + Math.max(0, now - since), since: null });
    this.now.set(now);
  }

  private resumeClock(): void {
    const { spent, since } = this.timer();
    if (since !== null) return;
    const now = Date.now();
    this.timer.set({ spent, since: now });
    this.now.set(now);
  }
}
