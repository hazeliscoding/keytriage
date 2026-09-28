// A guided test, run from its own events: which key to prompt, how many of its presses have
// counted, and where each round began and ended on the events' clock. The capture callback and a
// synthetic stream drive it the same way, so both give the same rounds.
use std::collections::{BTreeMap, BTreeSet};

use crate::input::{Device, Entry, HeldKey, Round};
use crate::keys;
use crate::params::PROMPT_MERGE_US;

// The largest plan a page may ask for. The longest test the app runs is 3 rounds of 30 presses,
// so anything past these is a mistaken request.
pub const MAX_ROUNDS: u16 = 10;
pub const MAX_PRESSES: u16 = 100;

// Every key once per round, in this order, for `rounds` rounds of `presses` presses each.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub keys: Vec<u16>,
    pub rounds: u16,
    pub presses: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanError {
    NoKeys,
    BadKey,
    RepeatedKey,
    Rounds,
    Presses,
    NoKeyboard,
    ZeroHandle,
}

// `round` and `index` count from 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Prompt {
    pub key: u16,
    pub count: u16,
    pub round: u16,
    pub index: u16,
}

// It holds release times, so it derives Debug only in tests.
#[cfg_attr(test, derive(Debug))]
pub struct Guide {
    plan: Plan,
    keyboard: Vec<Device>,
    step: u32,
    count: u16,
    start_us: u64,
    paused: bool,
    ended: bool,
    rounds: Vec<Round>,
    held: BTreeSet<u16>,
    last_up: BTreeMap<u16, u64>,
    trailing: BTreeSet<u16>,
    tallies: BTreeMap<u16, u32>,
    open_tally: u32,
}

// A round closes on its key's release, so a key that never sends one would stall the test. The
// engine drops media codes, rollover markers and Windows' fake shifts, so they could never count.
fn promptable(scan: u16) -> bool {
    let low = scan & 0xFF;
    low != 0
        && low != 0xFF
        && !keys::never_released(scan)
        && scan != keys::FAKE_LEFT_SHIFT
        && scan != keys::FAKE_RIGHT_SHIFT
}

impl Guide {
    pub fn new(plan: Plan, keyboard: &[Device]) -> Result<Guide, PlanError> {
        if plan.keys.is_empty() {
            return Err(PlanError::NoKeys);
        }
        if !plan.keys.iter().all(|&k| promptable(k)) {
            return Err(PlanError::BadKey);
        }
        if plan.keys.iter().collect::<BTreeSet<_>>().len() != plan.keys.len() {
            return Err(PlanError::RepeatedKey);
        }
        if !(1..=MAX_ROUNDS).contains(&plan.rounds) {
            return Err(PlanError::Rounds);
        }
        if !(1..=MAX_PRESSES).contains(&plan.presses) {
            return Err(PlanError::Presses);
        }
        if keyboard.is_empty() {
            return Err(PlanError::NoKeyboard);
        }
        // Injected input arrives on handle 0, and it must never answer a prompt.
        if keyboard.contains(&0) {
            return Err(PlanError::ZeroHandle);
        }
        Ok(Guide {
            plan,
            keyboard: keyboard.to_vec(),
            step: 0,
            count: 0,
            start_us: 0,
            paused: false,
            ended: false,
            rounds: Vec::new(),
            held: BTreeSet::new(),
            last_up: BTreeMap::new(),
            trailing: BTreeSet::new(),
            tallies: BTreeMap::new(),
            open_tally: 0,
        })
    }

    pub fn plan(&self) -> &Plan {
        &self.plan
    }

    pub fn paused(&self) -> bool {
        self.paused
    }

    fn steps(&self) -> u32 {
        self.plan.keys.len() as u32 * u32::from(self.plan.rounds)
    }

    pub fn prompt(&self) -> Option<Prompt> {
        if self.ended || self.step >= self.steps() {
            return None;
        }
        let n = self.plan.keys.len() as u32;
        let index = self.step % n;
        Some(Prompt {
            key: self.plan.keys[index as usize],
            count: self.count,
            round: (self.step / n) as u16,
            index: index as u16,
        })
    }

    // Progress through the plan: every closed or skipped round counts in full.
    pub fn done(&self) -> u32 {
        self.step.min(self.steps()) * u32::from(self.plan.presses) + u32::from(self.count)
    }

    pub fn total(&self) -> u32 {
        self.steps() * u32::from(self.plan.presses)
    }

    // Key-downs per key for the drawing, including ones too close to count toward a prompt.
    pub fn tallies(&self) -> &BTreeMap<u16, u32> {
        &self.tallies
    }

    // Returns whether the prompt, its count or a tally changed.
    pub fn entry(&mut self, entry: &Entry) -> bool {
        if self.prompt().is_none() {
            return false;
        }
        match *entry {
            Entry::Key {
                scan,
                up,
                device,
                micros,
            } => {
                if !self.accepts(device) {
                    return false;
                }
                if up {
                    self.release(scan, micros)
                } else {
                    self.press(scan, micros)
                }
            }
            Entry::Paused {
                ref interrupted, ..
            } => self.pause(interrupted),
            Entry::Resumed { micros } => {
                // The same key is asked for again, from 0, from the moment capture is back.
                if self.paused {
                    self.paused = false;
                    self.start_us = micros;
                }
                false
            }
        }
    }

    // A skip keeps the open round as asked, so a key that never registers gives silent rounds. While
    // paused there is no open round, so it only moves on, and the resume opens the next one.
    pub fn skip(&mut self, at_us: u64) -> bool {
        let Some(prompt) = self.prompt() else {
            return false;
        };
        if self.paused {
            self.step += 1;
        } else {
            let end = at_us.max(self.start_us);
            self.close(prompt.key, end);
            self.open(end);
        }
        true
    }

    // An open round that counted a press is kept, so a key held through its own round is still
    // evidence. One that counted none is dropped, so ending early never leaves a silent round.
    pub fn finish(&mut self, end_us: u64) -> Vec<Round> {
        if let Some(prompt) = self.prompt()
            && !self.paused
            && self.count > 0
        {
            self.close(prompt.key, end_us.max(self.start_us));
        }
        self.ended = true;
        std::mem::take(&mut self.rounds)
    }

    fn accepts(&self, device: Device) -> bool {
        device != 0 && self.keyboard.contains(&device)
    }

    fn press(&mut self, scan: u16, at: u64) -> bool {
        // Autorepeat and duplicate key-downs belong to the press that is already down.
        if !self.held.insert(scan) {
            return false;
        }
        let near = self
            .last_up
            .get(&scan)
            .is_some_and(|&up| at.saturating_sub(up) < PROMPT_MERGE_US);
        let trailing = self.trailing.remove(&scan) && near;
        let Some(prompt) = self.prompt() else {
            return false;
        };
        if self.paused {
            return false;
        }
        if scan == prompt.key {
            if at < self.start_us {
                return false;
            }
            // A key-down this close to the key's own release may be chatter, which must never
            // answer a prompt, or the round would end with presses the user never made.
            if !near && self.count < self.plan.presses {
                self.count += 1;
            }
            self.open_tally += 1;
            *self.tallies.entry(scan).or_default() += 1;
            return true;
        }
        // Chatter from a round's last press can land after the round has closed.
        if trailing {
            self.trailing.insert(scan);
            *self.tallies.entry(scan).or_default() += 1;
            return true;
        }
        false
    }

    fn release(&mut self, scan: u16, at: u64) -> bool {
        self.held.remove(&scan);
        self.last_up.insert(scan, at);
        if self.paused {
            return false;
        }
        let Some(prompt) = self.prompt() else {
            return false;
        };
        if scan != prompt.key || self.count < self.plan.presses {
            return false;
        }
        // Round::contains is half-open, and the release belongs to the round it ends.
        let end = at.saturating_add(1).max(self.start_us);
        self.close(scan, end);
        self.trailing.insert(scan);
        self.open(end);
        true
    }

    // A pause drops the open round, because presses made while the app was away were never seen.
    // Keys down at the pause stay held until their release, so their repeats after the resume
    // never count, and no gap is timed across the pause.
    fn pause(&mut self, interrupted: &[HeldKey]) -> bool {
        let changed = self.count > 0 || self.open_tally > 0;
        if let Some(prompt) = self.prompt()
            && let Some(tally) = self.tallies.get_mut(&prompt.key)
        {
            *tally = tally.saturating_sub(self.open_tally);
            if *tally == 0 {
                self.tallies.remove(&prompt.key);
            }
        }
        self.count = 0;
        self.open_tally = 0;
        self.paused = true;
        self.held = interrupted
            .iter()
            .filter(|h| self.accepts(h.device))
            .map(|h| h.scan)
            .collect();
        self.last_up.clear();
        self.trailing.clear();
        changed
    }

    fn close(&mut self, key: u16, end_us: u64) {
        self.rounds.push(Round {
            key,
            asked: self.plan.presses,
            start_us: self.start_us,
            end_us,
        });
    }

    fn open(&mut self, start_us: u64) {
        self.step += 1;
        self.count = 0;
        self.open_tally = 0;
        self.start_us = start_us;
    }
}
