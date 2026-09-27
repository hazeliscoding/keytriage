import { TestBed } from '@angular/core/testing';
import { App } from './app';

async function render(): Promise<HTMLElement> {
  const fixture = TestBed.createComponent(App);
  await fixture.whenStable();
  return fixture.nativeElement as HTMLElement;
}

function part(el: HTMLElement, heading: string): Element {
  const found = [...el.querySelectorAll('section')].find(
    (section) => section.querySelector('h3')?.textContent?.trim() === heading,
  );
  if (!found) throw new Error(`no section headed "${heading}"`);
  return found;
}

function lines(parent: Element, selector: string): string[] {
  return [...parent.querySelectorAll(selector)].map((line) => line.textContent?.trim() ?? '');
}

describe('App', () => {
  it('renders the app name', async () => {
    const el = await render();
    expect(el.querySelector('h1')?.textContent).toContain('keytriage');
  });

  it('says the finding is a sample', async () => {
    const el = await render();
    expect(el.textContent).toContain('This is a sample finding.');
  });

  it('shows the key with the finding and its confidence', async () => {
    const el = await render();
    expect(el.querySelector('h2')?.textContent?.trim()).toBe(
      'E: possible chatter, high confidence',
    );
  });

  it('shows the evidence', async () => {
    const el = await render();
    expect(lines(part(el, 'Evidence'), 'li')).toEqual([
      '14 of 100 presses sent an extra key-down 4 to 9 ms later',
      'reproduced in 3 of 3 rounds',
      'no neighbouring keys affected',
    ]);
  });

  it('ranks the likely causes', async () => {
    const el = await render();
    expect(lines(part(el, 'Likely causes'), 'ol > li')).toEqual([
      'switch contacts',
      'hot-swap socket or solder joint',
      'firmware debounce',
    ]);
  });

  it('gives the next test', async () => {
    const el = await render();
    expect(lines(part(el, 'Next test'), 'p')).toEqual([
      'Swap the E switch with the G switch and test both keys again.',
      'If the fault moves to G, the switch is the cause.',
      'If it stays on E, look at the socket or the PCB.',
    ]);
  });
});
