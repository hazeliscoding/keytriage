import { Component } from '@angular/core';

@Component({
  selector: 'app-root',
  template: '<h1>keytriage</h1>',
})
export class App {}

export const leak = () => fetch('https://example.com');
