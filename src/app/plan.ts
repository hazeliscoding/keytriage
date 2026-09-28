import { InjectionToken } from '@angular/core';
import { plainKeys, type Layout } from './layout';

// What the guided test asks for. It is data, so M4's swap retest and a later suspect-key picker can
// supply their own keys and rounds.
export interface Plan {
  keys: (drawn: Layout) => number[];
  rounds: number;
  presses: number;
}

// Every plain key of the chosen layout, 3 rounds of 10 presses (the owner's call, 2026.09.28).
export const DEFAULT_PLAN: Plan = { keys: plainKeys, rounds: 3, presses: 10 };

export const PLAN = new InjectionToken<Plan>('PLAN', { factory: () => DEFAULT_PLAN });
