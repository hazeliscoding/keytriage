import { Component, DestroyRef, computed, inject, signal } from '@angular/core';
import { stamp } from './format';
import { StartScreen } from './start-screen';
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
  imports: [StartScreen],
  host: { class: 'shell' },
})
export class App {
  protected readonly run = inject(TestRun);
  // Lasts for the session only: nothing is written anywhere until an export.
  protected readonly theme = signal<Theme>(systemTheme());
  // The Start screen's date and time. Only a new minute changes it.
  protected readonly now = signal(stamp(new Date()));

  protected readonly step = computed(() => {
    const screen = this.run.screen();
    return screen === 'test' ? 2 : screen === 'findings' ? 3 : 1;
  });
  protected readonly lockup = computed(() =>
    this.theme() === 'dark' ? 'brand/lockup-dark.svg' : 'brand/lockup.svg',
  );
  protected readonly capture = computed(() =>
    this.run.screen() === 'test' ? '● Capturing' : 'Capture off',
  );

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
