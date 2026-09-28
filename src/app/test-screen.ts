import { Component, computed, inject } from '@angular/core';
import { countWord, grouped, mmss } from './format';
import { KeyboardDrawing, type CapMark, type CapState } from './keyboard';
import { dropFocus } from './keys';
import { capLabel, capName } from './layout';
import { TestRun } from './test-run';

const SHOWN_ROWS = 16;

function listed(names: readonly string[]): string {
  return names.length < 2 ? names.join('') : `${names.slice(0, -1).join(', ')} and ${names.at(-1)}`;
}

@Component({
  selector: 'app-test-screen',
  templateUrl: './test-screen.html',
  imports: [KeyboardDrawing],
  // The buttons work by mouse only during a test, so a click never leaves one focused.
  host: { class: 'screen', '(click)': 'dropFocus()' },
})
export class TestScreen {
  protected readonly run = inject(TestRun);
  protected readonly dropFocus = dropFocus;

  // A click on the inactive window brings it back first, and capture resumes before the click
  // arrives, so a skip would close the resumed round as silent. A press that began during a focus
  // pause only brings the window back.
  private pressedAway = false;

  protected readonly rows = computed(() => this.run.rows().slice(0, SHOWN_ROWS));

  protected readonly prompt = computed(() => {
    const view = this.run.guide();
    if (!view || view.key === null) return null;
    const label = capLabel(this.run.layout(), view.key);
    const times = view.asked === 1 ? 'once' : `${countWord(view.asked)} times`;
    return { label, ask: `Press ${label} ${times}.`, count: view.count, asked: view.asked };
  });

  protected readonly progress = computed(() => {
    const view = this.run.guide();
    if (!view) return null;
    const injected = this.run.injected();
    return {
      round: `Round ${Math.min(view.round + 1, view.rounds)} of ${view.rounds}`,
      key: `Key ${Math.min(view.index + 1, view.keys)} of ${view.keys}`,
      fill: view.total ? (view.done / view.total) * 100 : 0,
      presses: injected
        ? `${grouped(injected)} injected, not counted`
        : `${grouped(view.done)} of ${grouped(view.total)} presses`,
    };
  });

  protected readonly elapsed = computed(
    () => `Elapsed ${mmss(this.run.elapsed())}${this.run.pause() ? ', paused' : ''}`,
  );

  // Later states win: a held prompted key loses its frame, and an interrupted key shows only that.
  protected readonly marks = computed(() => {
    const view = this.run.guide();
    const counts = new Map(view?.tallies.filter(([, n]) => n > 0));
    const marks = new Map<number, CapMark>();
    const mark = (scan: number, state: CapState, tag?: string) =>
      marks.set(scan, { state, count: counts.get(scan), tag });
    for (const scan of counts.keys()) mark(scan, 'counted');
    if (view && view.key !== null) mark(view.key, 'prompted');
    for (const scan of this.run.held()) mark(scan, 'down');
    for (const scan of this.run.interrupted()) mark(scan, 'interrupted', '○');
    return marks;
  });

  protected readonly pauseLabel = computed(() => {
    const pause = this.run.pause();
    return pause?.reason === 'focus' ? `PAUSED · WINDOW LOST FOCUS ${pause.at}` : 'PAUSED';
  });

  protected readonly pauseBody = computed(() => {
    const drawn = this.run.layout();
    const scans = this.run.interrupted();
    const labels = scans.map((scan) => capLabel(drawn, scan));
    // Two keys can share a label, such as both Alt keys, so those are told apart by name.
    const names = scans.map((scan, i) =>
      labels.indexOf(labels[i]) === labels.lastIndexOf(labels[i])
        ? labels[i]
        : capName(drawn, scan),
    );
    const lead =
      this.run.pause()?.reason === 'focus'
        ? 'Click back into the window to continue.'
        : 'Continue when ready.';
    const held =
      names.length === 0
        ? 'No key was down at that moment.'
        : names.length === 1
          ? `${names[0]} was down at that moment and is marked interrupted, not stuck.`
          : `${listed(names)} were down at that moment and are marked interrupted, not stuck.`;
    const prompt = this.prompt();
    const repeat = prompt ? ` This round of ${prompt.label} will be repeated.` : '';
    return `${lead} ${held}${repeat}`;
  });

  protected pressSkip(): void {
    this.pressedAway = this.run.pause()?.reason === 'focus';
  }

  protected skip(clicks: number): void {
    if (!this.pressedAway) void this.run.skipKey(clicks);
    this.pressedAway = false;
  }

  protected readonly injectedBody = computed(() => {
    const n = this.run.injected();
    return n === 1
      ? '1 event did not come from a physical keyboard. It stays in the event list, marked ' +
          'injected, and is left out of every count. Software such as macro tools or remote ' +
          'desktop can send them.'
      : `${grouped(n)} events did not come from a physical keyboard. They stay in the event ` +
          'list, marked injected, and are left out of every count. Software such as macro tools ' +
          'or remote desktop can send them.';
  });
}
