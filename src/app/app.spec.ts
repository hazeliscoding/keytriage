import { TestBed } from '@angular/core/testing';
import { App } from './app';

describe('App', () => {
  it('renders the app name', async () => {
    const fixture = TestBed.createComponent(App);
    await fixture.whenStable();
    const el = fixture.nativeElement as HTMLElement;
    expect(el.querySelector('h1')?.textContent).toContain('keytriage');
  });
});
