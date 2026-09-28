import { Component, computed, inject } from '@angular/core';
import { pad2 } from './format';
import { KeyboardDrawing, type CapMark } from './keyboard';
import { capLabel } from './layout';
import { TestRun } from './test-run';

@Component({
  selector: 'app-swap-screen',
  templateUrl: './swap-screen.html',
  imports: [KeyboardDrawing],
  host: { class: 'screen' },
})
export class SwapScreen {
  protected readonly run = inject(TestRun);

  // Rust chose the pair and wrote every sentence. The page names the two keys only in its kicker.
  protected readonly offer = computed(() => {
    const record = this.run.findings();
    const swap = record?.result.swap;
    if (!record || !swap) return null;
    return {
      a: capLabel(record.layout, swap.suspect),
      b: capLabel(record.layout, swap.partner),
      title: swap.title,
      knownGood: swap.knownGood,
      steps: swap.steps.map((text, i) => ({ num: pad2(i + 1), text })),
      means: swap.means,
    };
  });

  // The main test's keys, without counts, because the retest counts them again.
  protected readonly marks = computed(() => {
    const record = this.run.findings();
    const swap = record?.result.swap;
    const marks = new Map<number, CapMark>();
    if (!record || !swap) return marks;
    for (const { scan, count } of record.result.keys) {
      if (count > 0) marks.set(scan, { state: 'counted' });
    }
    marks.set(swap.suspect, { state: 'flagged', tag: 'A' });
    marks.set(swap.partner, { state: 'prompted', tag: 'B' });
    return marks;
  });
}
