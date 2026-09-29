import { Component, computed, input, output } from '@angular/core';
import { pickable, readingOrder, type Layout } from './layout';

// The owner's design shortens these labels on the picker only, so each fits its key.
const SHORT: Readonly<Record<string, string>> = {
  Backspace: 'Bksp',
  PrtSc: 'Prt',
  ScrLk: 'ScrL',
  Pause: 'Pse',
  Home: 'Hm',
  PgUp: 'PgU',
  PgDn: 'PgD',
  Enter: 'Ent',
  Shift: 'Shft',
  Caps: 'Cap',
  Menu: 'Mnu',
};

// Width in px of one key unit at most, from the design.
const MAX_UNIT = 76;

export interface Toggle {
  scan: number;
  clicks: number;
}

// Unlike the drawing, these keys are controls. Each key a test can prompt is a button named for its
// key, and the keys come in reading order, so Tab follows the rows. The others are only drawn.
@Component({
  selector: 'app-key-picker',
  templateUrl: './key-picker.html',
  host: {
    class: 'picker',
    role: 'group',
    '[style.aspect-ratio]': 'ratio()',
    '[style.width]': 'width()',
  },
})
export class KeyPicker {
  readonly layout = input.required<Layout>();
  readonly chosen = input.required<ReadonlySet<number>>();
  readonly toggled = output<Toggle>();

  protected readonly ratio = computed(() => `${this.layout().w} / ${this.layout().h}`);
  // The area is a size container, so the picker shrinks to its height as well as its width and
  // keeps the drawing's proportions.
  protected readonly width = computed(() => {
    const { w, h } = this.layout();
    return `min(100%, ${w * MAX_UNIT}px, calc(100cqh * ${w} / ${h}))`;
  });
  protected readonly keys = computed(() => {
    const drawn = this.layout();
    return readingOrder(drawn).map((cap) => ({
      cap,
      scan: pickable(cap) ? cap.scan : null,
      label: SHORT[cap.label] ?? cap.label,
      left: (cap.x / drawn.w) * 100,
      top: (cap.y / drawn.h) * 100,
      width: (cap.w / drawn.w) * 100,
      height: (cap.h / drawn.h) * 100,
    }));
  });
}
