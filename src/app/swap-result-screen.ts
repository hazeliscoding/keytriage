import { Component, computed, inject } from '@angular/core';
import { summaryOf } from './findings-screen';
import { KeyboardDrawing, type CapMark } from './keyboard';
import { capLabel } from './layout';
import { TestRun } from './test-run';

@Component({
  selector: 'app-swap-result-screen',
  templateUrl: './swap-result-screen.html',
  imports: [KeyboardDrawing],
  host: { class: 'screen' },
})
export class SwapResultScreen {
  protected readonly run = inject(TestRun);

  protected readonly card = computed(() => {
    const swapped = this.run.swapped();
    const record = this.run.findings();
    if (!swapped || !record) return null;
    const { outcome } = swapped;
    return {
      tile: outcome.tile === null ? '—' : capLabel(record.layout, outcome.tile),
      // Red belongs to a key that showed the fault. With none, the design draws the outcome where
      // neither did in green, and an unclear one stays neutral, as M3's nothing-tested card does.
      clean: outcome.tile === null && outcome.outcome === 'gone',
      none: outcome.tile === null && outcome.outcome !== 'gone',
      title: outcome.title,
      level: outcome.level === null ? null : `${outcome.level} confidence`,
      strong: outcome.strong,
      evidence: outcome.evidence,
      diagnosis: outcome.diagnosis,
      next: outcome.next,
      flagged: outcome.flagged.length > 0,
    };
  });

  // The strip describes the main test, as the design has it. The drawing shows the retest.
  protected readonly summary = computed(() => {
    const record = this.run.findings();
    return record ? summaryOf(record) : [];
  });

  // The result is one card, so every key that showed the fault carries its number.
  protected readonly marks = computed(() => {
    const swapped = this.run.swapped();
    const marks = new Map<number, CapMark>();
    if (!swapped) return marks;
    for (const { scan, count } of swapped.result.keys) {
      if (count > 0) marks.set(scan, { state: 'counted', count });
    }
    for (const scan of swapped.outcome.flagged) {
      marks.set(scan, { state: 'flagged', count: marks.get(scan)?.count, tag: '01' });
    }
    return marks;
  });
}
