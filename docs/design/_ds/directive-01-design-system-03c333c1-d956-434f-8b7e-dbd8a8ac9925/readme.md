# directive//01

A production-grade design system for institutional software: administration, case management, contracts, approvals, observation, and AI-agent orchestration. Its design logic is extracted from the presence of a calm, courteous, perfectly composed authority — Japanese government bureaucracy, controlled information access, hierarchy, contracts, and quiet observation. Conceptually inspired by the atmosphere around Makima (Chainsaw Man) without reproducing any copyrighted artwork or symbols.

**Central principle: the interface never raises its voice.** Power is expressed through certainty, restraint, hierarchy, precision, and space — never visual aggression. `TERMINATE CONTRACT` renders with the same composure as `SAVE CHANGES`; the difference is language, placement, authorization, and consequence.

Sources: none provided (from-scratch build from a written brief). No logo exists — render the wordmark `directive//01` in plain Directive type wherever a mark would go.

## The four layers
1. **CIVIL** — ordinary professional software: forms, records, calendars. Warm ivory, paper, fine borders. Feels surprisingly normal.
2. **AUTHORITY** — rank, permissions, approvals, delegation. Stronger black, wine red, numbering, formal labels. Order, not ornament.
3. **OBSERVATION** — monitoring, investigation, analytics. Concentric focus rings, timestamps, small metadata. Quiet, clinical, persistent.
4. **CONTROL** — contracts, overrides, termination. The interface becomes simpler and calmer as actions become more serious. Red appears — sparingly.

## CONTENT FUNDAMENTALS
- Voice: polite, short, calm, precise, confident. No exclamation marks, ever. No emoji, ever.
- Sentences are declarative and complete. "The request could not be completed. Review the conditions below." Never "Oh no! Something went wrong!"
- Restrictions state the requirement, not the failure: "Access is restricted. Authorization level 03 is required."
- Directive language (imperative, terse) is reserved for explicit authority: "Submit the report." "Remain available." "Return by 18:00." Never used for ordinary consumer interactions.
- Formal labels are UPPERCASE Directive type: `DIRECTIVE 071`, `SUBJECT 034`, `ACTIVE CONTRACT`, `LEVEL 03`.
- Body copy is sentence case. Buttons are short verbs: "Acknowledge", "Authorize", "Decline", "Terminate".
- Every consequential message names authority, consequence, and reversibility: "This action cannot be reversed." "This action will be recorded."
- Times are 24-hour (`18:00`); dates are `1997.09.14` dotted ISO order.
- The serif (Personal voice) appears only for private notes, quotations, and ceremonial text — never headings.

## VISUAL FOUNDATIONS
- **Color**: warm ivory/paper grounds (`--paper #f2ede2`, `--ivory`), ink black text (`--ink #1b1812`), graphite secondary. Character accents: auburn `#7e4a33` (links, focus, selection), copper, amber (attention/observation), wine `#63242c` (restriction), directive red `#9e2b2b` — used only when authority becomes visible (ACTIVE DIRECTIVE, OVERRIDE, TERMINATION). Most screens contain almost no red. Approval green `#4d6b50` is muted.
- **Dark mode** (`[data-theme="dark"]`): an empty government office after hours — warm brown-blacks (`#191712`), dim cream text, muted amber, dark burgundy. Never pure black, never neon.
- **Type**: three voices. OFFICIAL = Public Sans (interface, forms, tables). DIRECTIVE = Barlow Semi Condensed, semibold, uppercase, +0.08em tracking (commands, section titles, clearance labels). PERSONAL = Source Serif 4 (rare; notes, quotations). Understated hierarchy: 12px metadata, 13–14px body, 15–16px primary, 18–24px headings, 32–44px exceptional display only. No 96px type.
- **Spacing**: 12-step scale (2→96px). Generous margins around decisions; moderate density inside records; emptiness makes decisions heavier.
- **Shape**: rectangles, thin horizontal rules, precise circles. Radius 0–2px standard (`--radius-strict/standard`), 6px only on personal surfaces. No pills, no big rounded cards.
- **Focus Ring motif**: 2–3 concentric 1px circles at low opacity around avatars, selected entities, observed subjects. Auburn = selected, amber = active watch, red = escalated. Subtle, never a surveillance reticle. Decorative rings never replace the accessibility `:focus-visible` outline (2px auburn).
- **Borders**: 1px lines carry the whole system — `--border-paper` (hairline `#d3cab6`), `--border-record` (stronger), `--border-authority` (ink), `--border-control` (red). Shadows nearly absent: `--shadow-card` is 1px; overlays get one soft `--shadow-overlay`.
- **Backgrounds**: flat paper tones only. No gradients, no textures, no imagery, no blur/transparency effects.
- **Motion**: precise fades and short slides, 160ms standard / 240ms directive, cubic-bezier(0.4,0,0.2,1). No bounce, no spring, no overshoot. CONTROL-state changes are instant (`--motion-control: 0ms`) — instant is more authoritative.
- **Hover**: background shifts one paper step darker (`--bone`/`--bureau`) or ink buttons lighten to `--ink-soft`. Press: same, no shrink. Selection: auburn left rule or focus ring, not a filled highlight.
- **Tables**: fine 1px row rules, uppercase Directive column headers, extremely subtle alternating rows (ivory/paper), left-aligned text, tabular numerals.
- **Cards/panels**: ivory surface, 1px paper border, 0–2px radius, minimal shadow, uppercase Directive kicker label at top.

## ICONOGRAPHY
No proprietary icon set exists. Approach: thin, precise, institutional marks.
- Structural glyphs are preferred over pictograms: numbering (`01`), rules, `//`, `↓`, `↕`, `—`, `●`/`○` status dots, concentric ring SVGs (the Focus Ring primitive in `components/authority/FocusRing.jsx`).
- Where pictographic icons are required, use **Lucide** from CDN at 1.5px stroke, 16–18px — its thin precise line style matches. Flagged substitution: no source icon assets were provided.
- Status is never color-only: dots pair with uppercase text labels.
- Never: skulls, horns, literal chains, eye icons, emoji.

## Ethics
Control is the aesthetic, not the UX strategy. No dark patterns, false urgency, hidden opt-outs, or coercive defaults. Every sensitive action shows authority, consequence, reversibility, and audit ("This action will be recorded"). Observation UIs always show reason, authority, duration, review date. Permissions are presented as explicit contracts (purpose/scope/duration/storage) with equal-weight Decline.

## Index
- `styles.css` — global entry; imports everything in `tokens/` (colors incl. dark theme, typography, spacing, borders, motion, base).
- `guidelines/` — foundation specimen cards (Design System tab).
- `components/actions/` — Button, IconButton
- `components/forms/` — TextField, Select, Checkbox, Radio, Switch
- `components/display/` — Panel, Badge, Avatar, KeyValue, DataTable, Stat
- `components/feedback/` — Notice, EmptyState, Dialog
- `components/authority/` — FocusRing, AuthorityBadge, Directive, Contract, CommandChain, ApprovalChain, Observation, SubjectRecord, Handler, ConsequencePanel, AuditRecord, ClearanceGate
- `ui_kits/bureau/` — "Public Safety Office" administration shell: dashboard, personnel, subject record, directive, control state, AI-agent management.
- `SKILL.md` — agent skill entry point.

### Intentional additions
Standard form/action primitives (Button…Dialog) are authored because no source inventory exists; the brief's special components (Directive, Contract, etc.) are all built as specified.
