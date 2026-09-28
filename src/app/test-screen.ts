import { Component, computed, inject } from '@angular/core';
import { grouped } from './format';
import { KeyboardDrawing, type CapMark } from './keyboard';
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

  protected readonly rows = computed(() => this.run.rows().slice(0, SHOWN_ROWS));

  protected readonly marks = computed(() => {
    const marks = new Map<number, CapMark>();
    for (const scan of this.run.held()) marks.set(scan, { state: 'down' });
    for (const scan of this.run.interrupted()) marks.set(scan, { state: 'interrupted', tag: '○' });
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
    return `${lead} ${held}`;
  });

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
