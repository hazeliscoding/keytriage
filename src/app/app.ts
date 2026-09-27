import { Component } from '@angular/core';

@Component({
  selector: 'app-root',
  templateUrl: './app.html',
})
export class App {
  // Canned until the engine produces findings. It matches the planned report in README.md.
  protected readonly sample = {
    key: 'E',
    finding: 'possible chatter',
    confidence: 'high',
    evidence: [
      '14 of 100 presses sent an extra key-down 4 to 9 ms later',
      'reproduced in 3 of 3 rounds',
      'no neighbouring keys affected',
    ],
    causes: ['switch contacts', 'hot-swap socket or solder joint', 'firmware debounce'],
    nextTest: [
      'Swap the E switch with the G switch and test both keys again.',
      'If the fault moves to G, the switch is the cause.',
      'If it stays on E, look at the socket or the PCB.',
    ],
  };
}
