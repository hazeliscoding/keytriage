import { Component, computed, inject } from '@angular/core';
import { grouped, mmss, pad2, stamp } from './format';
import { KeyboardDrawing, type CapMark } from './keyboard';
import { capLabel } from './layout';
import { TestRun, type Findings } from './test-run';

const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;

// The main test's summary strip, which the swap result shows too.
export function summaryOf(record: Findings): { label: string; value: string }[] {
  return [
    { label: 'Keyboard', value: record.keyboard },
    { label: 'Layout', value: `${record.layout.size} ${record.layout.std}` },
    { label: 'Board', value: record.board },
    {
      label: 'Rounds',
      value: `${record.rounds} × ${plural(record.presses, 'press', 'presses')}`,
    },
    { label: 'Started', value: stamp(record.started) },
    { label: 'Duration', value: mmss(record.duration) },
  ];
}

@Component({
  selector: 'app-findings-screen',
  templateUrl: './findings-screen.html',
  imports: [KeyboardDrawing],
  host: { class: 'screen' },
})
export class FindingsScreen {
  protected readonly run = inject(TestRun);

  // Rust sends the findings ordered by confidence, so their numbers follow that order.
  protected readonly cards = computed(() => {
    const record = this.run.findings();
    if (!record) return [];
    return record.result.findings.map((finding, i) => {
      const most = Math.max(0, ...(finding.gaps ?? []).map((bar) => bar.count));
      return {
        num: pad2(i + 1),
        label: capLabel(record.layout, finding.key),
        title: finding.title,
        level: `${finding.level} confidence`,
        strong: finding.strong,
        evidence: finding.evidence,
        bars: finding.gaps?.map((bar) => ({
          ...bar,
          height: most ? Math.round((bar.count / most) * 100) : 0,
        })),
        causes: finding.causes.map((text, j) => ({ num: pad2(j + 1), text })),
        next: finding.next,
      };
    });
  });

  // A key named by several findings shows the first one's number.
  protected readonly marks = computed(() => {
    const record = this.run.findings();
    const marks = new Map<number, CapMark>();
    if (!record) return marks;
    for (const { scan, count } of record.result.keys) {
      if (count > 0) marks.set(scan, { state: 'counted', count });
    }
    record.result.findings.forEach((finding, i) => {
      const mark = marks.get(finding.key);
      if (mark?.state === 'flagged') return;
      marks.set(finding.key, { state: 'flagged', count: mark?.count, tag: pad2(i + 1) });
    });
    return marks;
  });

  protected readonly summary = computed(() => {
    const record = this.run.findings();
    return record ? summaryOf(record) : [];
  });

  // Rust lists every key that kept a round. With none, there is nothing a clean card could vouch for.
  protected readonly tested = computed(() => (this.run.findings()?.result.keys.length ?? 0) > 0);

  // A test that ended early says how far it got, so the card never claims the whole plan.
  protected readonly coverage = computed(() => {
    const record = this.run.findings();
    if (!record) return '';
    if (record.reached) {
      const { done, total } = record.reached;
      return (
        `Ended after ${grouped(done)} of ${grouped(total)} presses, ` +
        `${record.result.keys.length} of ${plural(record.keys, 'key', 'keys')}`
      );
    }
    const each = record.rounds === 1 ? 'in 1 round' : `in each of ${record.rounds} rounds`;
    return `${plural(record.keys, 'key', 'keys')}, ${plural(record.presses, 'press', 'presses')} ${each}`;
  });

  protected readonly cleanEvidence = computed(() => [
    this.coverage(),
    ...(this.run.findings()?.result.clean ?? []),
  ]);

  protected readonly limits = computed(() =>
    [
      'The test sees what the firmware reports after its own debounce.',
      this.run.findings()?.result.resolution,
      'A finding describes evidence and likelihood, not a verdict on a part.',
    ]
      .filter(Boolean)
      .join(' '),
  );
}
