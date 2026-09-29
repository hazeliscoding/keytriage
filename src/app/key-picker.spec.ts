import { TestBed } from '@angular/core/testing';
import { KeyPicker, type Toggle } from './key-picker';
import { layout, pickable, readingOrder, type Layout } from './layout';

const E = 0x12;
const R = 0x13;

async function draw(drawn: Layout, chosen: ReadonlySet<number> = new Set()) {
  const fixture = TestBed.createComponent(KeyPicker);
  fixture.componentRef.setInput('layout', drawn);
  fixture.componentRef.setInput('chosen', chosen);
  await fixture.whenStable();
  const host = fixture.nativeElement as HTMLElement;
  const buttons = [...host.querySelectorAll<HTMLButtonElement>('button.pick')];
  const scanOf = (node: Element) => Number(node.getAttribute('data-scan'));
  const find = (scan: number) => buttons.find((b) => scanOf(b) === scan) as HTMLButtonElement;
  return { fixture, host, buttons, scanOf, find };
}

describe('KeyPicker', () => {
  it('is a group of buttons for exactly the keys a test can prompt, in reading order', async () => {
    const full = layout('Full size', 'ISO');
    const { host, buttons, scanOf } = await draw(full);
    expect(host.getAttribute('role')).toBe('group');
    expect(buttons.map(scanOf)).toEqual(
      readingOrder(full)
        .filter(pickable)
        .map((cap) => cap.scan),
    );
    expect(host.querySelectorAll('.pick')).toHaveLength(full.caps.length);
    expect(buttons.slice(0, 2).map((b) => b.getAttribute('aria-label'))).toEqual(['Esc', 'F1']);
    expect(buttons.every((b) => b.type === 'button')).toBe(true);

    // 75% builds Home after the bottom row, and draws it at the end of the number row.
    const names = (await draw(layout('75%', 'ANSI'))).buttons.map((b) =>
      b.getAttribute('aria-label'),
    );
    expect(names.indexOf('Home')).toBe(names.indexOf('Backspace') + 1);
  });

  it("only draws the keys a test can't prompt, with no button, focus or name", async () => {
    const { host } = await draw(layout('Full size', 'ISO'));
    const fixed = [...host.querySelectorAll<HTMLElement>('.pick--fixed')];
    expect(fixed.map((node) => node.textContent?.trim())).toEqual([
      'Prt',
      'Pse',
      'Shft',
      'Shft',
      'Win',
      'Fn',
    ]);
    for (const node of fixed) {
      expect(node.tagName).toBe('DIV');
      expect(node.getAttribute('aria-hidden')).toBe('true');
      expect(node.hasAttribute('tabindex')).toBe(false);
      expect(node.querySelector('button')).toBeNull();
    }
  });

  it("names each key in full and prints the design's short labels", async () => {
    const { find } = await draw(layout('Full size', 'ANSI'));
    const shown = (scan: number) => [
      find(scan).textContent?.trim(),
      find(scan).getAttribute('aria-label'),
    ];
    expect(shown(0x0e)).toEqual(['Bksp', 'Backspace']);
    expect(shown(0xe038)).toEqual(['Alt', 'Right Alt']);
    expect(shown(0x1c)).toEqual(['Ent', 'Enter']);
    expect(shown(0xe01c)).toEqual(['Ent', 'Num Enter']);
    expect(shown(0x47)).toEqual(['7', 'Num 7']);
    expect(shown(0x39)).toEqual(['', 'Space']);
    expect(shown(E)).toEqual(['E', 'E']);
  });

  it('presses the chosen keys and reports each click with its count', async () => {
    const { fixture, find, buttons } = await draw(layout('75%', 'ANSI'), new Set([E]));
    expect(buttons.filter((b) => b.getAttribute('aria-pressed') === 'true')).toEqual([find(E)]);
    expect(find(E).classList).toContain('pick--chosen');
    expect(find(R).getAttribute('aria-pressed')).toBe('false');

    const seen: Toggle[] = [];
    fixture.componentInstance.toggled.subscribe((toggle) => seen.push(toggle));
    find(R).dispatchEvent(new MouseEvent('click', { bubbles: true, detail: 1 }));
    find(R).dispatchEvent(new MouseEvent('click', { bubbles: true, detail: 2 }));
    // Space and Enter on a focused button click it with no count.
    find(R).dispatchEvent(new MouseEvent('click', { bubbles: true, detail: 0 }));
    expect(seen).toEqual([
      { scan: R, clicks: 1 },
      { scan: R, clicks: 2 },
      { scan: R, clicks: 0 },
    ]);

    fixture.componentRef.setInput('chosen', new Set([R]));
    await fixture.whenStable();
    expect([find(E), find(R)].map((b) => b.getAttribute('aria-pressed'))).toEqual([
      'false',
      'true',
    ]);
  });
});
