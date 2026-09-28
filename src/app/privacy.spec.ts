import { type Provider } from '@angular/core';
import type { GuideView, TestResult } from './ipc';
import {
  GOLDEN,
  GOLDEN_PLAN,
  all,
  button,
  click,
  inApp,
  key,
  leaveApp,
  render,
  replay,
  text,
  type Emitted,
} from './testing/harness';

const G = 0x22;
const E = 0x12;
const OWN = 11;
const RESULT: TestResult = {
  rules: 2,
  findings: [],
  notes: [],
  clean: [],
  keys: [],
  resolution: '',
};

// The golden run holds only the tested keyboard's keys. This short test of G, 2 presses in 1 round,
// adds what it lacks, in the order Rust emits it: a press, injected input, another keyboard, a
// focus loss that repeats the round, then the presses that finish it. Rust's `done` includes the
// open round's count, and its tallies hold only keys that sent a key-down.
function view(count: number, done = count, next: number | null = G): GuideView {
  return {
    key: next,
    asked: 2,
    count,
    round: 0,
    rounds: 1,
    index: 0,
    keys: 1,
    done,
    total: 2,
    tallies: done ? [[G, done]] : [],
  };
}
const events = (...list: Parameters<typeof key>[]): Emitted[] =>
  list.map((args) => ({ event: 'test:event', payload: key(...args) }));
const guide = (payload: GuideView): Emitted => ({ event: 'test:guide', payload });
const SCRIPT: Emitted[] = [
  guide(view(0)),
  ...events([G, false, OWN, 1_000_000]),
  guide(view(1)),
  ...events([G, true, OWN, 1_060_000]),
  ...events([E, false, 0, 1_500_000], [E, true, 0, 1_550_000], [E, false, 21, 1_600_000]),
  ...events([0x38, false, OWN, 1_900_000]),
  // The pause lists every key still down, on any keyboard.
  {
    event: 'test:event',
    payload: {
      kind: 'paused',
      micros: 2_000_000,
      interrupted: [
        { device: OWN, scan: 0x38 },
        { device: 21, scan: E },
      ],
    },
  },
  guide(view(0)),
  { event: 'test:event', payload: { kind: 'resumed', micros: 3_000_000 } },
  ...events([G, false, OWN, 3_500_000]),
  guide(view(1)),
  ...events([G, true, OWN, 3_560_000], [G, false, OWN, 4_000_000]),
  guide(view(2)),
  ...events([G, true, OWN, 4_060_000]),
  guide(view(0, 2, null)),
];

const READABLE = [
  'key',
  'code',
  'keyCode',
  'which',
  'charCode',
  'location',
  'repeat',
  'isComposing',
  'altKey',
  'ctrlKey',
  'metaKey',
  'shiftKey',
];
const KEY_EVENTS = ['keydown', 'keyup', 'keypress'];

// A key event whose every way of telling the key apart records that it was read. jsdom's selector
// engine reads the key of every event to track :focus-visible, which a browser does natively, so
// its reads are not the page's.
function dispatchKey(type: string): { prevented: boolean; read: string[] } {
  const event = new KeyboardEvent(type, { bubbles: true, cancelable: true });
  const read: string[] = [];
  const record = (name: string) => {
    if (!new Error().stack?.includes('dom-selector')) read.push(name);
  };
  for (const name of READABLE) {
    Object.defineProperty(event, name, { get: () => record(name) });
  }
  Object.defineProperty(event, 'getModifierState', {
    value: () => {
      record('getModifierState');
      return false;
    },
  });
  document.body.dispatchEvent(event);
  return { prevented: event.defaultPrevented, read };
}

function dispatchKeys(): { prevented: boolean[]; read: string[] } {
  const results = KEY_EVENTS.map(dispatchKey);
  return { prevented: results.map((r) => r.prevented), read: results.flatMap((r) => r.read) };
}

const CONSOLE = ['debug', 'dir', 'error', 'info', 'log', 'table', 'trace', 'warn'] as const;

// Records every call that would store, log or add a history entry until the returned function
// stops watching and hands back what it saw.
function watchWrites(): () => string[] {
  const seen: string[] = [];
  const saw = (name: string) => () => void seen.push(name);
  const spies = [
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(saw('Storage.setItem')),
    vi.spyOn(History.prototype, 'pushState').mockImplementation(saw('history.pushState')),
    vi.spyOn(History.prototype, 'replaceState').mockImplementation(saw('history.replaceState')),
    vi.spyOn(Document.prototype, 'cookie', 'set').mockImplementation(saw('document.cookie')),
    ...CONSOLE.map((name) => vi.spyOn(console, name).mockImplementation(saw(`console.${name}`))),
  ];
  // jsdom has no IndexedDB, so a stand-in records any attempt to open one.
  const had = Object.getOwnPropertyDescriptor(window, 'indexedDB');
  Object.defineProperty(window, 'indexedDB', {
    configurable: true,
    value: { open: saw('indexedDB.open') },
  });
  return () => {
    for (const spy of spies) spy.mockRestore();
    if (had) Object.defineProperty(window, 'indexedDB', had);
    else delete (window as { indexedDB?: unknown }).indexedDB;
    return seen;
  };
}

describe('privacy', () => {
  beforeEach(() => inApp((cmd) => (cmd === 'end_test' ? RESULT : null)));
  afterEach(leaveApp);

  describe('keys', () => {
    it('cancels key events during a test without reading which key it was', async () => {
      const fixture = await render();
      expect(dispatchKeys()).toEqual({ prevented: [false, false, false], read: [] });
      await click(fixture, button(fixture, 'Begin test'));
      expect(dispatchKeys()).toEqual({ prevented: [true, true, true], read: [] });
      await click(fixture, button(fixture, 'End test'));
      expect(dispatchKeys()).toEqual({ prevented: [false, false, false], read: [] });
    });

    it('catches a listener that reads the key', async () => {
      const fixture = await render();
      await click(fixture, button(fixture, 'Begin test'));
      const peek = (event: Event) => void (event as KeyboardEvent).key;
      window.addEventListener('keydown', peek, true);
      try {
        expect(dispatchKey('keydown').read).toEqual(['key']);
      } finally {
        window.removeEventListener('keydown', peek, true);
      }
    });
  });

  describe('writes', () => {
    // Every write from Begin through `script` and the findings to an export.
    async function throughout(script: readonly Emitted[], providers: Provider[] = []) {
      const stop = watchWrites();
      let seen: string[];
      try {
        const fixture = await render(providers);
        await click(fixture, button(fixture, 'Begin test'));
        await replay(fixture, script);
        // A finished plan may move on by itself, which leaves no End test to click.
        const end = all(fixture, 'button').find((b) => text(b) === 'End test');
        if (end) await click(fixture, end);
        await click(fixture, button(fixture, 'Export report'));
      } finally {
        seen = stop();
      }
      return seen;
    }

    it('stores, logs and adds no history entries through the golden run', async () => {
      inApp((cmd) => {
        if (cmd === 'end_test') return GOLDEN.result;
        if (cmd === 'export_report') return 'keytriage-2026.09.28-1412.json';
        return null;
      }, GOLDEN.keyboards);
      expect(await throughout(GOLDEN.script, [GOLDEN_PLAN])).toEqual([]);
    });

    it('stores, logs and adds no history entries through a pause and foreign input', async () => {
      expect(await throughout(SCRIPT)).toEqual([]);
    });

    const forbidden: [string, () => unknown][] = [
      ['Storage.setItem', () => localStorage.setItem('k', 'v')],
      ['Storage.setItem', () => sessionStorage.setItem('k', 'v')],
      ['indexedDB.open', () => indexedDB.open('k')],
      ['history.pushState', () => history.pushState(null, '', '#k')],
      ['history.replaceState', () => history.replaceState(null, '', '#k')],
      [
        'document.cookie',
        () => {
          document.cookie = 'k=v';
        },
      ],
      ...CONSOLE.map((name): [string, () => unknown] => [
        `console.${name}`,
        () => console[name]('k'),
      ]),
    ];

    it.each(forbidden)('catches %s', (name, write) => {
      const stop = watchWrites();
      let seen: string[];
      try {
        write();
      } finally {
        seen = stop();
      }
      expect(seen).toEqual([name]);
    });
  });
});
