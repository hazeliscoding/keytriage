import { Component, computed, input } from '@angular/core';
import type { Layout } from './layout';

export type CapState = 'waiting' | 'counted' | 'prompted' | 'down' | 'interrupted' | 'flagged';

export interface CapMark {
  state: CapState;
  count?: number;
  tag?: string;
}

// A drawing, not a control: caps take no focus, clicks or keys.
@Component({
  selector: 'app-keyboard',
  templateUrl: './keyboard.html',
  host: {
    class: 'keyboard',
    '[style.aspect-ratio]': 'ratio()',
    '[style.max-width]': 'fit()',
  },
})
export class KeyboardDrawing {
  readonly layout = input.required<Layout>();
  readonly marks = input<ReadonlyMap<number, CapMark>>(new Map());

  protected readonly ratio = computed(() => `${this.layout().w} / ${this.layout().h}`);
  // Each screen sets --kb-reserve to the height its other rows need, so the drawing never
  // pushes the window into a scrollbar.
  protected readonly fit = computed(
    () => `calc((100vh - var(--kb-reserve, 0px)) * ${this.layout().w} / ${this.layout().h})`,
  );
  protected readonly caps = computed(() => {
    const { caps, w, h } = this.layout();
    return caps.map((cap) => ({
      cap,
      left: (cap.x / w) * 100,
      top: (cap.y / h) * 100,
      width: (cap.w / w) * 100,
      height: (cap.h / h) * 100,
    }));
  });
}
