import { Component, DestroyRef, computed, inject, signal } from '@angular/core';
import { FindingsScreen } from './findings-screen';
import { stamp } from './format';
import { StartScreen } from './start-screen';
import { SwapResultScreen } from './swap-result-screen';
import { SwapScreen } from './swap-screen';
import { TestScreen } from './test-screen';
import { TestRun } from './test-run';

type Theme = 'light' | 'dark';

function systemTheme(): Theme {
  return typeof matchMedia === 'function' && matchMedia('(prefers-color-scheme: dark)').matches
    ? 'dark'
    : 'light';
}

@Component({
  selector: 'app-root',
  templateUrl: './app.html',
  imports: [StartScreen, TestScreen, FindingsScreen, SwapScreen, SwapResultScreen],
  host: { class: 'shell' },
})
export class App {
  protected readonly run = inject(TestRun);
  // Lasts for the session only: nothing is written anywhere until an export.
  protected readonly theme = signal<Theme>(systemTheme());
  // The Start screen's date and time. Only a new minute changes it.
  protected readonly now = signal(stamp(new Date()));

  // The swap's screens belong to the findings, and its retest to the test.
  protected readonly step = computed(() => {
    switch (this.run.shown()) {
      case 'test':
        return 2;
      case 'findings':
      case 'swap':
      case 'swap-result':
        return 3;
      default:
        return 1;
    }
  });
  protected readonly lockup = computed(() =>
    this.theme() === 'dark' ? 'brand/lockup-dark.svg' : 'brand/lockup.svg',
  );
  protected readonly capture = computed(() => {
    if (this.run.screen() !== 'test') return 'off';
    return this.run.pause() ? 'paused' : 'on';
  });

  constructor() {
    this.applyTheme(this.theme());
    const tick = setInterval(() => this.now.set(stamp(new Date())), 1000);
    inject(DestroyRef).onDestroy(() => clearInterval(tick));
  }

  protected toggleTheme(): void {
    this.applyTheme(this.theme() === 'dark' ? 'light' : 'dark');
  }

  private applyTheme(theme: Theme): void {
    this.theme.set(theme);
    // On the root element, so the page background and scrollbars follow as well.
    document.documentElement.setAttribute('data-theme', theme);
  }
}
