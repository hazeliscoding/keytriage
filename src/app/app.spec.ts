import { TestBed, type ComponentFixture } from '@angular/core/testing';
import { mockIPC } from '@tauri-apps/api/mocks';
import { App } from './app';
import { RECONNECTED } from './ipc';
import { keyCount, layout, plainKeys } from './layout';
import { PLAN } from './plan';
import { TestRun } from './test-run';
import {
  EMPTY,
  FOLLOWS,
  all,
  button,
  chooseKeys,
  click,
  el,
  inApp,
  leaveApp,
  nthClick,
  offered,
  pickKey,
  render,
  sent,
  settle,
  started,
  text,
  textOf,
} from './testing/harness';

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

    it('reads 03 on both swap screens and 02 during the swap test', async () => {
      const fixture = await offered([FOLLOWS]);
      const step = () => text(el(fixture).querySelector('.steps__now'));
      expect(step()).toBe('03 Findings');
      await click(fixture, button(fixture, 'Run the swap test'));
      expect(step()).toBe('03 Findings');
      expect(all(fixture, '.header__end > span').map(text)).toEqual([
        'Synthetic keyboard · 0000:0001',
        '75% ANSI',
        'Hot-swap',
        'Capture off',
      ]);
      await click(fixture, button(fixture, 'Switches swapped. Test both keys'));
      expect(step()).toBe('02 Test');
      await click(fixture, button(fixture, 'End test'));
      expect(step()).toBe('03 Findings');
      expect(text(el(fixture).querySelector('.findings__list > .kicker'))).toBe(
        'Swap test // Result',
      );
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

    it('holds the keyboard, the layout and the board while a test starts', async () => {
      inApp((cmd) => (cmd === 'start_test' ? new Promise(() => undefined) : null));
      const fixture = await render();
      await click(fixture, button(fixture, 'Begin test'));
      await click(fixture, all(fixture, '.start__devices .row')[3]);
      await click(fixture, button(fixture, 'ISO'));
      await click(fixture, all(fixture, '.row--size')[4]);
      await click(fixture, all(fixture, 'input[name="board"]')[1] as HTMLInputElement);
      const picked = all(fixture, '.row--selected').map((row) =>
        text(row.querySelector('.row__name, .radio')),
      );
      expect(picked).toEqual(['HID Keyboard Device', '75%', 'Hot-swap']);
      expect(button(fixture, 'ANSI').getAttribute('aria-pressed')).toBe('true');
      expect(
        (all(fixture, 'input[name="board"]') as HTMLInputElement[]).map((r) => r.checked),
      ).toEqual([true, false, false]);
    });

    it('says when there is no keyboard to test, and keeps Begin test off', async () => {
      leaveApp();
      const fixture = await render();
      expect(all(fixture, '.start__devices .row')).toHaveLength(0);
      expect(text(el(fixture).querySelector('.rows__empty'))).toBe('No keyboard found.');
      expect(button(fixture, 'Begin test').disabled).toBe(true);
    });

    it('gives the reason when the keyboards cannot be listed', async () => {
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

    it('draws no keyboard on Start', async () => {
      const fixture = await render();
      const drawn = () => all(fixture, 'app-start-screen app-keyboard, app-start-screen .cap');
      for (const std of ['ANSI', 'ISO']) {
        await click(fixture, button(fixture, std));
        for (let i = 0; i < 5; i++) {
          await click(fixture, all(fixture, '.row--size')[i]);
          expect(drawn()).toHaveLength(0);
        }
      }
      await click(fixture, button(fixture, 'Begin test'));
      expect(all(fixture, 'app-test-screen app-keyboard .cap')).toHaveLength(
        keyCount('60%', 'ISO'),
      );
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

  describe('scope', () => {
    const E = 0x12;
    const R = 0x13;
    const T = 0x14;
    const HOME = 0xe047;
    const FOOTNOTE =
      'Click keys on the drawing to add or remove them. Use this when you already know which ' +
      "keys misbehave. Win, PrtSc, Shift, Fn and Pause can't be chosen: Win and PrtSc open " +
      'Windows features, Shift opens Sticky Keys, and Fn and Pause send nothing the test can time.';
    const NEIGHBOR =
      'Choose a neighboring key too, so a dead key can be told from a silent keyboard.';
    const summary = (fixture: ComponentFixture<App>) => textOf(fixture, '.choose__summary');
    const hint = (fixture: ComponentFixture<App>) => textOf(fixture, '.choose__hint');
    const shown = (fixture: ComponentFixture<App>, label: string) =>
      all(fixture, 'button').some((b) => text(b) === label);

    it('shows 04 // Scope under the columns, with All keys pressed', async () => {
      const fixture = await render();
      const row = el(fixture).querySelector('.start > .scope');
      expect(row?.previousElementSibling?.classList).toContain('start__column');
      expect(text(row?.querySelector('.kicker'))).toBe('04 // Scope');
      const scopes = [button(fixture, 'All keys'), button(fixture, 'Chosen keys')];
      expect(scopes.map((b) => b.getAttribute('aria-pressed'))).toEqual(['true', 'false']);
      expect(scopes.map((b) => b.classList.contains('btn--primary'))).toEqual([true, false]);
      expect(text(row?.querySelector('.scope__end'))).toBe(
        'Every letter, digit and punctuation key, 10 presses in each of 3 rounds. Choose keys ' +
          'instead when you already know which ones misbehave.',
      );
      expect(all(fixture, 'app-key-picker')).toHaveLength(0);
    });

    it('swaps the columns for the picker and back, focusing the pressed scope', async () => {
      const fixture = await render();
      await click(fixture, button(fixture, 'Chosen keys'));
      expect(all(fixture, '.start__column')).toHaveLength(0);
      expect(textOf(fixture, '.choose > .scope .kicker')).toBe('04 // Scope');
      const scopes = [button(fixture, 'All keys'), button(fixture, 'Chosen keys')];
      expect(scopes.map((b) => b.getAttribute('aria-pressed'))).toEqual(['false', 'true']);
      expect(scopes.map((b) => b.classList.contains('btn--primary'))).toEqual([false, true]);
      expect(document.activeElement).toBe(scopes[1]);
      // The header's date and time, repeated as the design does.
      expect(textOf(fixture, '.choose .scope__end')).toBe(textOf(fixture, '.header__context'));
      expect(textOf(fixture, '#choose-title')).toBe('Click the keys to test.');
      const picker = el(fixture).querySelector('app-key-picker');
      expect(picker?.getAttribute('aria-labelledby')).toBe('choose-title');
      expect(all(fixture, '.pick')).toHaveLength(keyCount('75%', 'ANSI'));
      expect(textOf(fixture, '.choose__lead')).toBe(FOOTNOTE);
      expect(summary(fixture)).toBe('None chosen');
      expect(shown(fixture, 'Clear')).toBe(false);
      expect(button(fixture, 'Begin test').disabled).toBe(true);

      await click(fixture, button(fixture, 'All keys'));
      expect(all(fixture, '.start__column')).toHaveLength(3);
      expect(all(fixture, 'app-key-picker')).toHaveLength(0);
      expect(document.activeElement).toBe(button(fixture, 'All keys'));
      expect(button(fixture, 'Begin test').disabled).toBe(false);
    });

    it("toggles a key per single click, keeps reading order and drops a double click's second", async () => {
      const fixture = await render();
      await chooseKeys(fixture, T, E, R);
      expect(summary(fixture)).toBe('3 chosen · E R T');
      const pressed = () =>
        all(fixture, '.pick[aria-pressed="true"]').map((b) => Number(b.getAttribute('data-scan')));
      expect(pressed()).toEqual([E, R, T]);
      await pickKey(fixture, E);
      expect(summary(fixture)).toBe('2 chosen · R T');
      const e = el(fixture).querySelector(`.pick[data-scan="${E}"]`) as HTMLElement;
      await nthClick(fixture, e, 2);
      expect(summary(fixture)).toBe('2 chosen · R T');
      // Space or Enter on a focused key clicks it with no count.
      await nthClick(fixture, e, 0);
      expect(summary(fixture)).toBe('3 chosen · E R T');
    });

    it('asks for a neighboring key while one key is chosen', async () => {
      const fixture = await render();
      await chooseKeys(fixture, E);
      expect([summary(fixture), hint(fixture)]).toEqual(['1 chosen · E', NEIGHBOR]);
      await pickKey(fixture, R);
      expect([summary(fixture), hint(fixture)]).toEqual(['2 chosen · E R', '']);
      await click(fixture, button(fixture, 'Clear'));
      expect([summary(fixture), hint(fixture)]).toEqual(['None chosen', '']);
    });

    it('names keys whose label is blank or shared as the design does', async () => {
      const fixture = await render();
      await chooseKeys(fixture, 0x39, 0xe038, 0x38, 0x1d);
      expect(summary(fixture)).toBe('4 chosen · LCtrl LAlt Space RAlt');
    });

    it('clears the keys chosen, then hides Clear and focuses Chosen keys', async () => {
      const fixture = await render();
      await chooseKeys(fixture, E, R);
      expect(button(fixture, 'Clear').classList).toContain('btn--ghost');
      await click(fixture, button(fixture, 'Clear'));
      expect(summary(fixture)).toBe('None chosen');
      expect(shown(fixture, 'Clear')).toBe(false);
      expect(document.activeElement).toBe(button(fixture, 'Chosen keys'));
      expect(button(fixture, 'Begin test').disabled).toBe(true);
      await TestBed.inject(TestRun).begin();
      expect(started()).toEqual([]);
    });

    it("does nothing for a key the test can't prompt", async () => {
      const fixture = await render();
      await chooseKeys(fixture, E);
      const fixed = all(fixture, '.pick--fixed');
      expect(fixed.map(text)).toEqual(['Prt', 'Shft', 'Shft', 'Win', 'Fn']);
      for (const drawn of fixed) await nthClick(fixture, drawn, 1);
      expect(summary(fixture)).toBe('1 chosen · E');
      // The run refuses them too, whatever asks.
      const run = TestBed.inject(TestRun);
      for (const scan of [0xe05b, 0xe037, 0x2a, 0x36, 0xe11d]) run.toggleKey(scan);
      run.toggleKey(R);
      await settle(fixture);
      expect(summary(fixture)).toBe('2 chosen · E R');
      expect([...run.chosen()]).toEqual([E, R]);
    });

    it('tests the chosen keys in reading order at 3 × 30, and again from the findings', async () => {
      const fixture = await render();
      await click(fixture, all(fixture, '.start__devices .row')[3]);
      await chooseKeys(fixture, E);
      expect(button(fixture, 'Test 1 key').disabled).toBe(false);
      await pickKey(fixture, T);
      await pickKey(fixture, R);
      await click(fixture, button(fixture, 'Test 3 keys'));
      const plan = { keyboard: [21, 22, 23], keys: [E, R, T], rounds: 3, presses: 30 };
      expect(started()).toEqual([{ ...plan, board: 'hot-swap' }]);
      await click(fixture, button(fixture, 'End test'));
      await click(fixture, button(fixture, 'Test again'));
      expect(started()).toEqual([plan, plan].map((p) => ({ ...p, board: 'hot-swap' })));
      await click(fixture, button(fixture, 'End test'));
      await click(fixture, button(fixture, 'New test'));
      expect(summary(fixture)).toBe('3 chosen · E R T');
    });

    it('leaves out a chosen key the layout lacks, and brings it back', async () => {
      const fixture = await render();
      await chooseKeys(fixture, E, HOME);
      expect(summary(fixture)).toBe('2 chosen · Home E');
      const pick = async (size: number) => {
        await click(fixture, button(fixture, 'All keys'));
        await click(fixture, all(fixture, '.row--size')[size]);
        await click(fixture, button(fixture, 'Chosen keys'));
      };
      await pick(3);
      expect(summary(fixture)).toBe('1 chosen · E');
      await click(fixture, button(fixture, 'Test 1 key'));
      expect(started().map((plan) => plan.keys)).toEqual([[E]]);
      await click(fixture, button(fixture, 'End test'));
      await click(fixture, button(fixture, 'New test'));
      await pick(2);
      expect(summary(fixture)).toBe('2 chosen · Home E');
    });

    it('holds the scope and the choice while a test starts, so Test again repeats it', async () => {
      let answer = (): void => undefined;
      inApp((cmd) => {
        if (cmd === 'end_test') return EMPTY;
        if (cmd !== 'start_test' || started().length > 1) return null;
        return new Promise((done) => (answer = () => done(null)));
      });
      const fixture = await render();
      await chooseKeys(fixture, E, R);
      await click(fixture, button(fixture, 'Test 2 keys'));
      await pickKey(fixture, T);
      await click(fixture, button(fixture, 'Clear'));
      await click(fixture, button(fixture, 'All keys'));
      expect(summary(fixture)).toBe('2 chosen · E R');
      answer();
      await settle(fixture);
      await click(fixture, button(fixture, 'End test'));
      await click(fixture, button(fixture, 'Test again'));
      const plan = { keys: [E, R], rounds: 3, presses: 30 };
      expect(started().map(({ keys, rounds, presses }) => ({ keys, rounds, presses }))).toEqual([
        plan,
        plan,
      ]);
    });

    it('still asks for every plain key at 3 × 10 under All keys', async () => {
      const fixture = await render();
      await chooseKeys(fixture, E, R);
      await click(fixture, button(fixture, 'All keys'));
      await click(fixture, button(fixture, 'Begin test'));
      const [plan] = started();
      expect(plan.keys).toEqual(plainKeys(layout('75%', 'ANSI')));
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
      expect(sent('list_keyboards')).toHaveLength(1);
    });

    it('lists the keyboards again when the picked one was reconnected', async () => {
      inApp((cmd) => {
        if (cmd === 'start_test') throw RECONNECTED;
        return null;
      });
      const fixture = await render();
      expect(sent('list_keyboards')).toHaveLength(1);
      await click(fixture, button(fixture, 'Begin test'));
      expect(text(el(fixture).querySelector('.footer__note'))).toBe(RECONNECTED);
      expect(text(el(fixture).querySelector('.steps__now'))).toBe('01 Keyboard');
      expect(sent('list_keyboards')).toHaveLength(2);
    });
  });

  describe('status', () => {
    // A live region inserted together with its text is often not read, so the region must be on
    // the page before the text it announces.
    it('announces each note from one region that stays on the page across screens', async () => {
      inApp((cmd) => {
        if (cmd === 'start_test') throw RECONNECTED;
        return null;
      });
      const fixture = await render();
      const [region] = all(fixture, '[role="status"]');
      expect(all(fixture, '[role="status"]')).toHaveLength(1);
      expect(region.classList).toContain('visually-hidden');
      expect(text(region)).toBe('');
      await click(fixture, button(fixture, 'Begin test'));
      expect(all(fixture, '[role="status"]')).toEqual([region]);
      expect(text(region)).toBe(RECONNECTED);
      expect(el(fixture).querySelector('.footer__note')?.hasAttribute('role')).toBe(false);
    });
  });
});
