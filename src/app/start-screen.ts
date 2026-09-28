import { Component, computed, inject } from '@angular/core';
import { KeyboardDrawing } from './keyboard';
import { SIZES, STDS, keyCount } from './layout';
import { BOARDS, TestRun } from './test-run';

interface EntryRow {
  handle: number;
  name: string;
  id: string;
  meta: string;
  child: boolean;
}

@Component({
  selector: 'app-start-screen',
  templateUrl: './start-screen.html',
  imports: [KeyboardDrawing],
  host: { class: 'screen' },
})
export class StartScreen {
  protected readonly run = inject(TestRun);
  protected readonly stds = STDS;
  protected readonly boards = BOARDS;

  // A group's first entry heads it, and its other collections are indented under it.
  protected readonly rows = computed<EntryRow[]>(() =>
    (this.run.groups() ?? []).flatMap((group) => {
      const n = group.entries.length;
      const head = group.builtIn
        ? n > 1
          ? `Built in · ${n} entries`
          : 'Built in'
        : n > 1
          ? `${n} entries, one device`
          : 'One entry';
      return group.entries.map((entry, i) => ({
        handle: entry.handle,
        name: entry.name,
        id: entry.id ?? '—',
        meta: i === 0 ? head : group.builtIn ? 'Built in' : 'Same device',
        child: i > 0,
      }));
    }),
  );

  protected readonly sizes = computed(() =>
    SIZES.map((size) => ({ size, keys: keyCount(size, this.run.std()) })),
  );

  constructor() {
    void this.run.listKeyboards();
  }
}
