import { InjectionToken } from '@angular/core';
import { chosenKeys, plainKeys, type Layout } from './layout';

// What the main test asks for: all keys or the chosen ones. Rust builds the swap retest's plan from
// its own offer.
export interface Plan {
  keys: (drawn: Layout) => number[];
  rounds: number;
  presses: number;
}

// Every plain key of the chosen layout, 3 rounds of 10 presses (the owner's call, 2026.09.28).
export const DEFAULT_PLAN: Plan = { keys: plainKeys, rounds: 3, presses: 10 };

// The swap retest's size, the owner's call for chosen keys (2026.09.28): they are keys the user
// already suspects, and 90 clean presses bound a key's rate at 3.4%. testing/choosing.json pins both
// to the engine.
export const CHOSEN_ROUNDS = 3;
export const CHOSEN_PRESSES = 30;

export function chosenPlan(chosen: ReadonlySet<number>): Plan {
  return {
    keys: (drawn) => chosenKeys(drawn, chosen),
    rounds: CHOSEN_ROUNDS,
    presses: CHOSEN_PRESSES,
  };
}

export const PLAN = new InjectionToken<Plan>('PLAN', { factory: () => DEFAULT_PLAN });
