import { Injectable } from '@angular/core';
import { invoke, type InvokeArgs } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Events, KeyName, KeyboardGroup, PlanArgs, TestResult } from './ipc';

export const OUTSIDE_APP = 'Not running in the app.';

// Checked at each call rather than once, so the page still renders in a plain browser
// (npm start) and specs can install or remove the mock between tests.
function inApp(): boolean {
  const internals = (window as { __TAURI_INTERNALS__?: { invoke?: unknown } }).__TAURI_INTERNALS__;
  return typeof internals?.invoke === 'function';
}

function call<T>(cmd: string, args?: InvokeArgs): Promise<T> {
  // A command's Err reaches the page as a bare string, so this one does too.
  return inApp() ? invoke<T>(cmd, args) : Promise.reject(OUTSIDE_APP);
}

export function reasonOf(error: unknown): string {
  if (typeof error === 'string') return error;
  if (error instanceof Error) return error.message;
  return 'The app did not say why.';
}

// The only code that talks to Rust.
@Injectable({ providedIn: 'root' })
export class Bridge {
  listKeyboards(): Promise<KeyboardGroup[]> {
    return inApp() ? invoke<KeyboardGroup[]>('list_keyboards') : Promise.resolve([]);
  }

  startTest(plan: PlanArgs): Promise<void> {
    return call('start_test', { plan });
  }

  pauseTest(): Promise<void> {
    return call('pause_test');
  }

  continueTest(): Promise<void> {
    return call('continue_test');
  }

  skipKey(round: number, index: number): Promise<void> {
    return call('skip_key', { round, index });
  }

  endTest(labels: KeyName[]): Promise<TestResult> {
    return call('end_test', { labels });
  }

  exportReport(name: string): Promise<string | null> {
    return call('export_report', { name });
  }

  on<K extends keyof Events>(event: K, handler: (payload: Events[K]) => void): Promise<() => void> {
    if (!inApp()) return Promise.resolve(() => undefined);
    return listen<Events[K]>(event, (message) => handler(message.payload));
  }
}
