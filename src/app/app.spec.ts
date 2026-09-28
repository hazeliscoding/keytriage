import { type Provider } from '@angular/core';
import { TestBed, type ComponentFixture } from '@angular/core/testing';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { App } from './app';
import type { KeyboardGroup, PlanArgs } from './ipc';
import { keyCount, layout, plainKeys } from './layout';
import { PLAN } from './plan';

type Call = [string, unknown];

const GROUPS: KeyboardGroup[] = [
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

let calls: Call[];

// Every command succeeds unless `answer` throws for it.
function inApp(answer: (cmd: string) => unknown = () => null): void {
  calls = [];
  mockIPC(
    (cmd, args) => {
      calls.push([cmd, args]);
      return cmd === 'list_keyboards' ? GROUPS : answer(cmd);
    },
    { shouldMockEvents: true },
  );
}

function leaveApp(): void {
  clearMocks();
  // clearMocks keeps the object itself, which a real page outside the app never has.
  delete (window as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
}

async function settle(fixture: ComponentFixture<App>): Promise<void> {
  await new Promise((done) => setTimeout(done));
  await fixture.whenStable();
}

async function render(providers: Provider[] = []): Promise<ComponentFixture<App>> {
  TestBed.configureTestingModule({ providers });
  const fixture = TestBed.createComponent(App);
  await settle(fixture);
  return fixture;
}

function el(fixture: ComponentFixture<App>): HTMLElement {
  return fixture.nativeElement as HTMLElement;
}

function all(fixture: ComponentFixture<App>, selector: string): HTMLElement[] {
  return [...el(fixture).querySelectorAll<HTMLElement>(selector)];
}

function text(node: Element | null | undefined): string {
  return node?.textContent?.replace(/\s+/g, ' ').trim() ?? '';
}

function button(fixture: ComponentFixture<App>, label: string): HTMLButtonElement {
  const found = all(fixture, 'button').find((b) => text(b) === label);
  if (!found) throw new Error(`no button "${label}"`);
  return found as HTMLButtonElement;
}

function started(): PlanArgs[] {
  return calls
    .filter(([cmd]) => cmd === 'start_test')
    .map(([, args]) => (args as { plan: PlanArgs }).plan);
}

async function click(fixture: ComponentFixture<App>, target: HTMLElement): Promise<void> {
  target.click();
  await settle(fixture);
}

describe('App', () => {
  beforeEach(() => inApp());

  afterEach(() => {
    leaveApp();
    document.documentElement.removeAttribute('data-theme');
  });

  describe('header', () => {
    it('shows the lockup, the first step, the date and that capture is off', async () => {
      const fixture = await render();
      expect(el(fixture).querySelector('.header__lockup')?.getAttribute('src')).toBe(
        'brand/lockup.svg',
      );
      expect(text(el(fixture).querySelector('.steps__now'))).toBe('01 Keyboard');
      expect(text(el(fixture).querySelector('.header__context'))).toMatch(
        /^\d{4}\.\d{2}\.\d{2} \d{2}:\d{2}$/,
      );
      expect(text(el(fixture).querySelector('.header__capture'))).toBe('Capture off');
    });

    it('switches the theme and the lockup, and names the theme it switches to', async () => {
      const fixture = await render();
      expect(document.documentElement.getAttribute('data-theme')).toBe('light');
      await click(fixture, button(fixture, 'Dark'));
      expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
      expect(el(fixture).querySelector('.header__lockup')?.getAttribute('src')).toBe(
        'brand/lockup-dark.svg',
      );
      await click(fixture, button(fixture, 'Light'));
      expect(document.documentElement.getAttribute('data-theme')).toBe('light');
    });
  });

  describe('keyboard column', () => {
    it("lists each device's entries under it, with their IDs", async () => {
      const fixture = await render();
      const rows = all(fixture, '.start__devices .row');
      expect(rows.map((row) => text(row.querySelector('.row__name')))).toEqual([
        'HID Keyboard Device',
        'Keychron K2',
        'Keychron K2',
        'Keychron K2',
        'Standard PS/2 Keyboard',
      ]);
      expect(rows.map((row) => text(row.querySelector('.row__id')))).toEqual([
        '046D:C52B',
        '3434:0220',
        '3434:0220',
        '3434:0220',
        '—',
      ]);
      expect(rows.map((row) => text(row.querySelector('.row__meta')))).toEqual([
        'One entry',
        '3 entries, one device',
        'Same device',
        'Same device',
        'Built in',
      ]);
      expect(rows.map((row) => row.classList.contains('row--child'))).toEqual([
        false,
        false,
        true,
        true,
        false,
      ]);
    });

    it('preselects the first keyboard', async () => {
      const fixture = await render();
      const rows = all(fixture, '.start__devices .row');
      expect(rows.map((row) => row.getAttribute('aria-pressed'))).toEqual([
        'true',
        'false',
        'false',
        'false',
        'false',
      ]);
      expect(rows[0].classList).toContain('row--selected');
    });

    it("starts the test on every entry of the picked entry's device, with the plan", async () => {
      const fixture = await render([
        { provide: PLAN, useValue: { keys: () => [0x22, 0x24, 0x12], rounds: 3, presses: 10 } },
      ]);
      await click(fixture, all(fixture, '.start__devices .row')[3]);
      const picked = all(fixture, '.row--selected').map((row) =>
        text(row.querySelector('.row__name, .radio')),
      );
      expect(picked).toEqual(['Keychron K2', '75%', 'Hot-swap']);
      await click(fixture, button(fixture, 'Begin test'));
      expect(started()).toEqual([
        {
          keyboard: [21, 22, 23],
          keys: [0x22, 0x24, 0x12],
          rounds: 3,
          presses: 10,
          board: 'hot-swap',
        },
      ]);
    });

    it('says when there is no keyboard to test, and keeps Begin test off', async () => {
      leaveApp();
      const fixture = await render();
      expect(all(fixture, '.start__devices .row')).toHaveLength(0);
      expect(text(el(fixture).querySelector('.rows__empty'))).toBe('No keyboard found.');
      expect(button(fixture, 'Begin test').disabled).toBe(true);
    });

    it('gives the reason when the keyboards cannot be listed', async () => {
      calls = [];
      mockIPC(() => {
        throw 'Windows did not list the keyboards.';
      });
      const fixture = await render();
      expect(text(el(fixture).querySelector('.rows__empty'))).toBe('No keyboard found.');
      expect(text(el(fixture).querySelector('.footer__note'))).toBe(
        'Windows did not list the keyboards.',
      );
      expect(button(fixture, 'Begin test').disabled).toBe(true);
    });
  });

  describe('layout column', () => {
    it('counts one more key on every size for ISO', async () => {
      const fixture = await render();
      const counts = () => all(fixture, '.row--size .row__keys').map(text);
      expect(counts()).toEqual(['104 keys', '87 keys', '83 keys', '68 keys', '61 keys']);
      expect(button(fixture, 'ANSI').classList).toContain('btn--primary');
      await click(fixture, button(fixture, 'ISO'));
      expect(counts()).toEqual(['105 keys', '88 keys', '84 keys', '69 keys', '62 keys']);
      expect(button(fixture, 'ISO').classList).toContain('btn--primary');
      expect(button(fixture, 'ANSI').classList).toContain('btn--secondary');
    });

    it('previews the chosen layout, outlines only', async () => {
      const fixture = await render();
      const preview = () => all(fixture, '.keyboard--bare .cap');
      expect(preview()).toHaveLength(keyCount('75%', 'ANSI'));
      await click(fixture, all(fixture, '.row--size')[4]);
      expect(preview()).toHaveLength(keyCount('60%', 'ANSI'));
      expect(all(fixture, '.keyboard--bare .cap__label')).toHaveLength(0);
    });

    it('asks for every plain key of the chosen layout by default', async () => {
      const fixture = await render();
      await click(fixture, button(fixture, 'ISO'));
      await click(fixture, all(fixture, '.row--size')[0]);
      await click(fixture, button(fixture, 'Begin test'));
      const [plan] = started();
      expect(plan.keys).toEqual(plainKeys(layout('Full size', 'ISO')));
      expect(plan.keys).toHaveLength(48);
      expect([plan.rounds, plan.presses]).toEqual([3, 10]);
    });
  });

  describe('board column', () => {
    it('preselects hot-swap and sends the board picked', async () => {
      const fixture = await render();
      const radios = all(fixture, 'input[name="board"]') as HTMLInputElement[];
      expect(radios.map((radio) => radio.checked)).toEqual([true, false, false]);
      expect(
        all(fixture, '.row--board').map((row) => text(row.querySelector('.row__about'))),
      ).toEqual([
        'Switches pull out. The swap test is available.',
        'Switches are fixed to the PCB. Next steps skip the swap.',
        'Built in. Next steps cover the keycap, the scissor and the membrane.',
      ]);
      await click(fixture, radios[1]);
      expect(all(fixture, '.row--board')[1].classList).toContain('row--selected');
      await click(fixture, button(fixture, 'Begin test'));
      expect(started()[0].board).toBe('soldered');
    });
  });

  describe('Begin test', () => {
    it('moves to the test and names the keyboard, layout and board in the header', async () => {
      const fixture = await render();
      await click(fixture, button(fixture, 'Begin test'));
      expect(text(el(fixture).querySelector('.steps__now'))).toBe('02 Test');
      expect(all(fixture, '.header__end > span').map(text)).toEqual([
        'HID Keyboard Device · 046D:C52B',
        '75% ANSI',
        'Hot-swap',
        '● Capturing',
      ]);
    });

    it('stays on Start with the reason when the test cannot start', async () => {
      inApp((cmd) => {
        if (cmd === 'start_test') throw 'Capture could not start.';
        return null;
      });
      const fixture = await render();
      await click(fixture, button(fixture, 'Begin test'));
      expect(text(el(fixture).querySelector('.footer__note'))).toBe('Capture could not start.');
      expect(text(el(fixture).querySelector('.steps__now'))).toBe('01 Keyboard');
      expect(button(fixture, 'Begin test').disabled).toBe(false);
    });
  });
});
