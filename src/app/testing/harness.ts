// Helpers for the specs that drive the whole page through a mocked bridge. Only specs import this.
import { type Provider } from '@angular/core';
import { TestBed, type ComponentFixture } from '@angular/core/testing';
import { emit } from '@tauri-apps/api/event';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { App } from '../app';
import type { Events, KeyName, KeyboardGroup, PlanArgs, TestEvent, TestResult } from '../ipc';
import { PLAN } from '../plan';
import golden from './guided-chatter.json';

export type Call = [string, unknown];

export const GROUPS: KeyboardGroup[] = [
  {
    name: 'HID Keyboard Device',
    id: '046D:C52B',
    builtIn: false,
    entries: [{ handle: 11, name: 'HID Keyboard Device', id: '046D:C52B' }],
  },
  {
    name: 'Keychron K2',
    id: '3434:0220',
    builtIn: false,
    entries: [
      { handle: 21, name: 'Keychron K2', id: '3434:0220' },
      { handle: 22, name: 'Keychron K2', id: '3434:0220' },
      { handle: 23, name: 'Keychron K2', id: '3434:0220' },
    ],
  },
  {
    name: 'Standard PS/2 Keyboard',
    id: null,
    builtIn: true,
    entries: [{ handle: 31, name: 'Standard PS/2 Keyboard', id: null }],
  },
];

// What end_test returns for a test with nothing to report.
export const EMPTY: TestResult = {
  rules: 2,
  findings: [],
  notes: [],
  clean: [],
  keys: [],
  resolution: '',
};

let log: Call[] = [];

// Every command the page sent since the mock was installed, with its arguments.
export function calls(): Call[] {
  return log;
}

export function sent(cmd: string): unknown[] {
  return log.filter(([name]) => name === cmd).map(([, args]) => args);
}

// Every command succeeds unless `answer` throws for it.
export function inApp(
  answer: (cmd: string, args: unknown) => unknown = (cmd) => (cmd === 'end_test' ? EMPTY : null),
  groups: KeyboardGroup[] = GROUPS,
): void {
  log = [];
  mockIPC(
    (cmd, args) => {
      log.push([cmd, args]);
      return cmd === 'list_keyboards' ? groups : answer(cmd, args);
    },
    { shouldMockEvents: true },
  );
}

export function leaveApp(): void {
  clearMocks();
  // clearMocks keeps the object itself, which a real page outside the app never has.
  delete (window as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
}

export async function settle(fixture: ComponentFixture<App>): Promise<void> {
  await new Promise((done) => setTimeout(done));
  await fixture.whenStable();
}

export async function render(providers: Provider[] = []): Promise<ComponentFixture<App>> {
  TestBed.configureTestingModule({ providers });
  const fixture = TestBed.createComponent(App);
  await settle(fixture);
  return fixture;
}

export function el(fixture: ComponentFixture<App>): HTMLElement {
  return fixture.nativeElement as HTMLElement;
}

export function all(fixture: ComponentFixture<App>, selector: string): HTMLElement[] {
  return [...el(fixture).querySelectorAll<HTMLElement>(selector)];
}

export function text(node: Element | null | undefined): string {
  return node?.textContent?.replace(/\s+/g, ' ').trim() ?? '';
}

export function textOf(fixture: ComponentFixture<App>, selector: string): string {
  return text(el(fixture).querySelector(selector));
}

export function button(fixture: ComponentFixture<App>, label: string): HTMLButtonElement {
  const found = all(fixture, 'button').find((b) => text(b) === label);
  if (!found) throw new Error(`no button "${label}"`);
  return found as HTMLButtonElement;
}

export async function click(fixture: ComponentFixture<App>, target: HTMLElement): Promise<void> {
  target.click();
  await settle(fixture);
}

export function started(): PlanArgs[] {
  return sent('start_test').map((args) => (args as { plan: PlanArgs }).plan);
}

// Renders the page in the app and begins a test on the first keyboard, 75% ANSI.
export async function testing(providers: Provider[] = []): Promise<ComponentFixture<App>> {
  const fixture = await render(providers);
  await click(fixture, button(fixture, 'Begin test'));
  return fixture;
}

// Delivers events as Rust emits them, in order, then lets the page render.
export async function send<K extends keyof Events>(
  fixture: ComponentFixture<App>,
  event: K,
  ...payloads: Events[K][]
): Promise<void> {
  for (const payload of payloads) await emit(event, payload);
  await fixture.whenStable();
}

// One emitted event, in the shape the golden script uses.
export type Emitted = { [K in keyof Events]: { event: K; payload: Events[K] } }[keyof Events];

export interface Golden {
  source: string;
  plan: PlanArgs;
  keyboards: KeyboardGroup[];
  labels: KeyName[];
  script: Emitted[];
  result: TestResult;
}

// The engine's synthetic chatter run as Rust emits it. src-tauri/src/golden.rs writes the file from
// the real engine and fails while it is out of date, so a spec that replays it follows Rust.
export const GOLDEN = golden as Golden;

// The golden run's three keys in place of every plain key of the layout.
export const GOLDEN_PLAN: Provider = {
  provide: PLAN,
  useValue: {
    keys: () => GOLDEN.plan.keys,
    rounds: GOLDEN.plan.rounds,
    presses: GOLDEN.plan.presses,
  },
};

export async function replay(
  fixture: ComponentFixture<App>,
  script: readonly Emitted[],
): Promise<void> {
  for (const { event, payload } of script) await emit(event, payload);
  await fixture.whenStable();
}

export function key(scan: number, up: boolean, device: number, micros: number): TestEvent {
  return { kind: 'key', scan, up, device, micros };
}

// A press and its release, 60 ms apart.
export function press(scan: number, device: number, micros: number): TestEvent[] {
  return [key(scan, false, device, micros), key(scan, true, device, micros + 60_000)];
}

// The drawn cap for a scan code.
export function cap(fixture: ComponentFixture<App>, scan: number): HTMLElement {
  const found = el(fixture).querySelector<HTMLElement>(
    `.keyboard:not(.keyboard--bare) .cap[data-scan="${scan}"]`,
  );
  if (!found) throw new Error(`no cap for ${scan}`);
  return found;
}
