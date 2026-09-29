import { TestBed } from '@angular/core/testing';
import { emit } from '@tauri-apps/api/event';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { Bridge, OUTSIDE_APP, reasonOf } from './bridge';
import type { GuideView, PlanArgs, TestResult } from './ipc';

type Call = [string, unknown];

const PLAN: PlanArgs = {
  keyboard: [7, 9],
  keys: [0x22, 0x24, 0x12],
  rounds: 3,
  presses: 10,
  board: 'hot-swap',
};
const RESULT: TestResult = {
  rules: 5,
  findings: [],
  notes: [],
  clean: [],
  keys: [],
  resolution: '',
  swap: null,
  outcome: null,
};
const VIEW: GuideView = {
  key: 0x12,
  asked: 10,
  count: 4,
  round: 0,
  rounds: 3,
  index: 0,
  keys: 3,
  done: 4,
  total: 90,
  tallies: [[0x12, 4]],
};

function leaveApp(): void {
  clearMocks();
  // clearMocks keeps the object itself, which a real page outside the app never has.
  delete (window as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
}

describe('Bridge', () => {
  let bridge: Bridge;

  beforeEach(() => {
    leaveApp();
    bridge = TestBed.inject(Bridge);
  });

  afterEach(leaveApp);

  describe('outside the app', () => {
    it('lists no keyboards', async () => {
      await expect(bridge.listKeyboards()).resolves.toEqual([]);
    });

    it('refuses every other command with a reason', async () => {
      await expect(bridge.startTest(PLAN)).rejects.toBe(OUTSIDE_APP);
      await expect(bridge.startSwapTest()).rejects.toBe(OUTSIDE_APP);
      await expect(bridge.endTest([])).rejects.toBe(OUTSIDE_APP);
      await expect(bridge.exportReport('keytriage-2026.09.28-1402.json')).rejects.toBe(OUTSIDE_APP);
      expect(OUTSIDE_APP).toBe('Not running in the app.');
    });

    it('listens to nothing without failing', async () => {
      const stop = await bridge.on('test:event', () => undefined);
      expect(() => stop()).not.toThrow();
    });
  });

  describe('in the app', () => {
    let calls: Call[];
    let swapReply: () => number[];

    beforeEach(() => {
      calls = [];
      swapReply = () => [41, 42];
      mockIPC(
        (cmd, args) => {
          calls.push([cmd, args]);
          if (cmd === 'list_keyboards') return [];
          if (cmd === 'end_test') return RESULT;
          if (cmd === 'export_report') return 'keytriage-2026.09.28-1402.json';
          if (cmd === 'start_swap_test') return swapReply();
          if (cmd === 'skip_key') throw 'No test is running.';
          return null;
        },
        { shouldMockEvents: true },
      );
    });

    it('sends each command with its arguments', async () => {
      await bridge.listKeyboards();
      await bridge.startTest(PLAN);
      await bridge.pauseTest();
      await bridge.continueTest();
      await expect(bridge.endTest([{ scan: 0x12, name: 'E' }])).resolves.toEqual(RESULT);
      await expect(bridge.exportReport('keytriage-2026.09.28-1402.json')).resolves.toBe(
        'keytriage-2026.09.28-1402.json',
      );
      expect(calls).toEqual([
        ['list_keyboards', {}],
        ['start_test', { plan: PLAN }],
        ['pause_test', {}],
        ['continue_test', {}],
        ['end_test', { labels: [{ scan: 0x12, name: 'E' }] }],
        ['export_report', { name: 'keytriage-2026.09.28-1402.json' }],
      ]);
    });

    it('starts the swap test with no arguments and returns the handles Rust used', async () => {
      await expect(bridge.startSwapTest()).resolves.toEqual([41, 42]);
      expect(calls).toEqual([['start_swap_test', {}]]);
    });

    it('passes on the reason Rust gives', async () => {
      const error = await bridge.skipKey(0, 0).catch((e: unknown) => e);
      expect(reasonOf(error)).toBe('No test is running.');
      swapReply = () => {
        throw 'There is no swap test to run.';
      };
      await expect(bridge.startSwapTest()).rejects.toBe('There is no swap test to run.');
      expect(reasonOf(new Error('Broken pipe'))).toBe('Broken pipe');
    });

    it('hands each event payload to its listener', async () => {
      const seen: GuideView[] = [];
      const stop = await bridge.on('test:guide', (view) => seen.push(view));
      await emit('test:guide', VIEW);
      await emit('test:guide', { ...VIEW, count: 5 });
      expect(seen).toEqual([VIEW, { ...VIEW, count: 5 }]);
      expect(() => stop()).not.toThrow();
    });
  });
});
