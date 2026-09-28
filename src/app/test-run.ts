import { Injectable, computed, inject, signal } from '@angular/core';
import { Bridge, reasonOf } from './bridge';
import type { Board, KeyboardGroup, PlanArgs } from './ipc';
import { layout, type Size, type Std } from './layout';
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
    this.screen.set('starting');
    try {
      await this.bridge.startTest(plan);
      if (this.screen() === 'starting') this.screen.set('test');
    } catch (error) {
      if (this.screen() !== 'starting') return;
      this.screen.set(screen);
      this.note.set(reasonOf(error));
    }
  }
}
