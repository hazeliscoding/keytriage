import {
  Component,
  Injector,
  afterNextRender,
  computed,
  inject,
  input,
  viewChild,
  type ElementRef,
} from '@angular/core';
import { KeyPicker } from './key-picker';
import { SIZES, STDS, capTag, keyCount } from './layout';
import { BOARDS, TestRun, type Scope } from './test-run';

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
  imports: [KeyPicker],
  host: { class: 'screen' },
})
export class StartScreen {
  protected readonly run = inject(TestRun);
  private readonly injector = inject(Injector);
  protected readonly stds = STDS;
  protected readonly boards = BOARDS;
  // The header's date and time, which the chosen view repeats on its Scope row.
  readonly now = input('');

  private readonly scopeAll = viewChild<ElementRef<HTMLButtonElement>>('scopeAll');
  private readonly scopeChosen = viewChild<ElementRef<HTMLButtonElement>>('scopeChosen');

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

  protected readonly summary = computed(() => {
    const drawn = this.run.layout();
    const keys = this.run.chosenKeys();
    if (!keys.length) return 'None chosen';
    return `${keys.length} chosen · ${keys.map((scan) => capTag(drawn, scan)).join(' ')}`;
  });

  protected readonly beginLabel = computed(() => {
    const n = this.run.chosenKeys().length;
    if (this.run.scope() !== 'chosen' || !n) return 'Begin test';
    return n === 1 ? 'Test 1 key' : `Test ${n} keys`;
  });

  protected readonly beginOff = computed(
    () =>
      !this.run.group() ||
      this.run.screen() === 'starting' ||
      (this.run.scope() === 'chosen' && !this.run.chosenKeys().length),
  );

  constructor() {
    void this.run.listKeyboards();
  }

  // Each scope draws its own Scope row, so the button pressed is gone once the view changes. Focus
  // moves to the new row's pressed button.
  protected setScope(scope: Scope): void {
    if (this.run.setScope(scope)) this.focusScope();
  }

  // Clear hides itself.
  protected clear(): void {
    if (this.run.clearKeys()) this.focusScope();
  }

  private focusScope(): void {
    afterNextRender(
      () => {
        const pressed = this.run.scope() === 'chosen' ? this.scopeChosen() : this.scopeAll();
        pressed?.nativeElement.focus();
      },
      { injector: this.injector },
    );
  }
}
