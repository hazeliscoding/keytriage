// The common result. Every number a finding cites is a field, so M3 can draw it and M4 can compare
// it. words.rs renders the fallback English.
use crate::aggregate::Aggregates;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Confidence {
    Low,
    Medium,
    High,
    VeryHigh,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Chatter,
    Dead,
    Stuck,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum PollEstimate {
    #[default]
    Unknown,
    AtMost4Ms,
    Ms8,
    Ms16OrSlower,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cap {
    CoarsePolling,
    Systemic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpanMs {
    pub min_ms: u32,
    pub max_ms: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChatterEvidence {
    pub presses: u32,
    pub affected: u32,
    pub extra_downs: u32,
    pub rate_floor_permille: u16,
    pub rounds: u16,
    pub rounds_affected: u16,
    pub gap: Option<SpanMs>,
    pub fragment: Option<SpanMs>,
    pub other_tested: u16,
    pub other_affected: u16,
    pub borderline: u32,
    pub timing_unknown: u32,
    pub poll: PollEstimate,
    pub cap: Option<Cap>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeadEvidence {
    pub rounds: u16,
    pub silent_rounds: u16,
    pub asked_in_silent: u32,
    pub control_rounds: u16,
    pub bracketed: bool,
    pub other_presses_meanwhile: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StuckEvidence {
    pub held_ms: u32,
    pub still_down: bool,
    pub episodes: u16,
    pub repeats: u32,
    pub others_completed: u32,
    pub own_prompts: u16,
    pub other_rounds_answered: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Evidence {
    Chatter(ChatterEvidence),
    Dead(DeadEvidence),
    Stuck(StuckEvidence),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Cause {
    SwitchContacts,
    HotSwapSocket,
    SolderJoint,
    SocketOrSolderJoint,
    SwitchSeating,
    DiodeOrTrace,
    FirmwareDebounce,
    KeymapOrRemap,
    DebrisOrResidue,
    DomeOrScissor,
    StemOrKeycap,
    LostRelease,
    HostSoftware,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NextTest {
    TestAgain { key: u16, rounds: u16, presses: u16 },
    AskBoardKind,
    SwapSwitch { suspect: u16, partner: Option<u16> },
    ReseatSwitch { key: u16 },
    BridgeSocket { key: u16 },
    CleanContacts { key: u16 },
    RaiseDebounce,
    InspectUnderKeycap { key: u16 },
    InspectSolderJoint { key: u16 },
    CheckKeymap { key: u16 },
    CheckConnection,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub key: u16,
    pub confidence: Confidence,
    pub evidence: Evidence,
    pub causes: Vec<Cause>,
    pub next_tests: Vec<NextTest>,
}

impl Finding {
    pub fn kind(&self) -> Kind {
        match self.evidence {
            Evidence::Chatter(_) => Kind::Chatter,
            Evidence::Dead(_) => Kind::Dead,
            Evidence::Stuck(_) => Kind::Stuck,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Why {
    Paused,
    Blocked,
    Injected,
    OtherKeyboard,
    HeldDown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Note {
    NoInputFromKeyboard,
    Systemic {
        keys: u16,
    },
    Clean {
        key: u16,
        presses: u32,
        bound_permille: u16,
    },
    OneExtraDown {
        key: u16,
        presses: u32,
        extra_downs: u32,
    },
    Unprompted {
        key: u16,
        affected: u32,
        presses: u32,
    },
    DifferentCode {
        asked: u16,
        got: u16,
        rounds: u16,
    },
    NotAssessed {
        key: u16,
        rounds: u16,
        why: Why,
    },
    HeldAtPause {
        key: u16,
        held_ms: u32,
    },
    ReleasedWithNextKey {
        key: u16,
        held_ms: u32,
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Limits {
    pub poll: PollEstimate,
    pub injected: u32,
    pub other_devices: u32,
    pub unknown_codes: u32,
    pub fake_shifts: u32,
    pub overruns: u32,
    pub pauses: u32,
    pub timing_unknown: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    pub rules: u16,
    pub findings: Vec<Finding>,
    pub notes: Vec<Note>,
    pub aggregates: Aggregates,
}
