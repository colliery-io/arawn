---
id: tui-visual-coherence-pass-palette
level: initiative
title: "TUI visual coherence pass — palette, hierarchy, tool-call rendering"
short_code: "ARAWN-I-0036"
created_at: 2026-05-06T10:42:48.922737+00:00
updated_at: 2026-05-06T10:42:48.922737+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: M
initiative_id: tui-visual-coherence-pass-palette
---

# TUI visual coherence pass

## Context

The TUI has solid technical bones — centralized `theme.rs`, owned wrap (`wrap.rs`), real markdown parser with syntect highlighting (`markdown.rs`), modal infra (`modal.rs`) — but visually it reads as *assembled* rather than *designed*. A senior visual review against the codebase surfaced a coherent set of issues that compound:

1. **`theme.rs` is decorative.** Defined constants are not actually imported by `render.rs` or `markdown.rs` — every caller hardcodes `Color::Cyan`, `Color::Rgb(100,100,115)`, `Color::DarkGray` etc. as literals. The "centralized theme" can't be changed in one place because nothing reads from it. Highest single leverage point.

2. **Color overload.** Cyan = focused border, tool name, H1 heading, link, sidebar-collapsed indicator, modal selection indicator. Six meanings → no meaning. The eye gets no help.

3. **Five drifting "dim grays".** `Rgb(100,100,115)`, `Rgb(150,150,165)`, `Rgb(140,140,155)`, plus `DarkGray` (named ANSI), plus `Rgb(180,180,195)`. All within ~30 luminance units of each other; no system. A coherent product picks ~12 named shades and uses them everywhere.

4. **Visual hierarchy is inverted.** Tool-call cards get full bordered boxes (4–6 lines per call). Assistant prose — the agent's actual answer, the most important content — gets a 2-space indent and reads as leftover terminal output. With 10 tool calls in a row, 50+ lines of mostly chrome bury the answer.

5. **Heading colors aren't a system.** H1=Cyan, H2=Blue, H3=Magenta, H4=White. Three accent colors with no luminance order. The eye doesn't read this as "deeper = smaller."

6. **Sidebar tab strip stipple is noisy.** The collapsed sidebar renders alternating `· ` rows that dominate every snapshot — 22 of 24 rows of stipple — drawing attention to a passive affordance instead of the conversation.

7. **Multiple drift-and-mismatch issues** in box-drawing chrome (top vs bottom width disagreement, byte-vs-display-width measurement) — covered in the bug-fix bundle but rooted in the same lack-of-system.

The visual layer is good enough to ship today, but it caps how the product feels. For a personal-assistant TUI being used hours a day, "feels designed" matters — it's the difference between a tool people open every morning and a tool people respect but rarely return to.

## Goals & Non-Goals

**Goals:**
- A named 12-step palette wired through `theme.rs` and used by every render call site.
- Each color has exactly one job. Cyan → "focused/active interactive" only. Status colors (success/warning/danger) consistent everywhere.
- Visual hierarchy: assistant prose is structurally strongest; tool-call chrome is structurally weakest. Inverse of today.
- Tool-call rendering collapses to a single line by default; expands on error or on demand.
- Empty states (first run, idle, generating, modal pending) are designed moments, not accidental.
- Markdown rendering quality matches the bar set by lazygit / glow / claude-code (table borders, code blocks, inline code, headings).

**Non-Goals:**
- Light-mode theme. Defer until the dark-mode palette is solid.
- Configurable themes. One opinionated palette ships first.
- Replacing the rendering engine (ratatui). It's the right choice; this is a coherence pass, not a rewrite.
- The personal-assistant dashboard surfaces (briefing widget, action-item panel). Those are I-0035's territory; this initiative makes them look right when they land.

## Requirements

### User Requirements
- New users perceive arawn as a designed product within ~5 seconds, not "assembled."
- Long sessions remain readable — chat content dominates, chrome recedes.
- Users with custom terminal palettes (gruvbox, solarized, dracula) don't see clashes between hard-RGB and ANSI-named colors.
- Color contrast against typical dark terminal backgrounds passes a WCAG-ish smell test (no DarkGray-on-default-dark unreadability).

### System Requirements
- **REQ-001:** `theme.rs` exposes a 12-step gray scale plus named accent + semantic colors (`success`, `warning`, `danger`, `accent`, `muted`, `border`, `border_focused`, `fg_muted`, `fg`, `fg_strong`, `bg`, `bg_subtle`, `bg_muted`).
- **REQ-002:** `render.rs` and `markdown.rs` import from `theme.rs` only — zero hardcoded `Color::*` or `Color::Rgb(...)` outside the theme module.
- **REQ-003:** Tool-call rendering has two modes: `collapsed` (single line, default) and `expanded` (current bordered card, on error or Ctrl+E).
- **REQ-004:** Heading hierarchy uses a monochrome luminance ladder + weight, not three accent hues. H1 bold White (or top-of-scale gray), H2 bold default, H3 bold dim, H4 italic dim.
- **REQ-005:** Sidebar collapsed strip is a single border + cyan `▸` mid-point. No stipple.
- **REQ-006:** Inline code renders without literal backticks (background already disambiguates).
- **REQ-007:** All chrome uses display-width (unicode-width crate) for measurement, not byte length, so unicode in tool names doesn't break alignment.
- **REQ-008:** Status bar redesign — see I-0035 (token-usage cluster moves to `/diag`). This initiative provides the visual treatment for whatever status-bar surface I-0035 lands on.
- **NFR-001:** No regression in render performance. The wrap pipeline + per-message cache from the prior render rewrite stay intact.
- **NFR-002:** Snapshot tests get re-baselined; visual regressions caught by future changes via `cargo insta`.

## Architecture

### Palette layer (`theme.rs`)

12-step gray scale (gray.1 = darkest bg-subtle, gray.12 = brightest fg-strong) named after the Radix Colors convention. Plus accent + status colors:

```
gray.1   bg_subtle            (rarely used, almost-black)
gray.2   bg                   (chat area background)
gray.3   bg_muted             (sidebar, status bar, modal bg)
gray.4   border               (chrome lines, dividers)
gray.5   border_subtle        (inter-row separators)
gray.6   border_focused       — replace with `accent` semantic
gray.7   fg_muted             (timestamps, descriptions, hints)
gray.8   fg_dim               (less-important content)
gray.9   fg                   (default chat text)
gray.10  fg_strong            (headings, focused labels)
gray.11  fg_emphasis          (rare; bold call-outs)
gray.12  fg_max               (white, sparingly)

accent           (cyan, but ONLY for "focused/active interactive")
success          (green — completion, success result)
warning          (yellow — in-progress spinner, attention)
danger           (red — errors, BYPASS mode)
code_bg          (bg for inline + block code)
```

All hex/Rgb values defined once. Every callsite uses these names.

### Rendering layer (`render.rs`, `markdown.rs`, `modal.rs`)

- Mechanical pass replacing `Color::*` literals with `theme::*` consts.
- Tool-call collapsed renderer: `⏵ shell · ls -la · 1.2s` on success, `✗ shell · ls -la · permission denied` on error. Ctrl+E swaps to the existing bordered card.
- Heading style table in `markdown.rs` rewritten for the luminance ladder.
- Sidebar tab strip: drop stipple loop; render single border + accent `▸`.
- Inline code: use `code_bg` only, no literal backticks.

### Chrome correctness

- Replace every `header_text.len()` measurement with `unicode_width::UnicodeWidthStr::width(...)`.
- Compute box width once, share between top and bottom borders.

## Detailed Design

### Phase 1 — Palette + theme wiring (the leverage move)

- Define the 12-step palette + semantic colors in `theme.rs`.
- Mechanical replacement pass through `render.rs` and `markdown.rs`. Every `Color::*` and `Color::Rgb(...)` becomes a `theme::*` reference.
- Snapshot tests will fail because RGB values shift slightly (some colors moving up/down a luminance step). Re-baseline via `cargo insta`.
- ~200 LOC.

### Phase 2 — Tool-call collapsed rendering

- Add `ChatRole::ToolCall` rendering with two modes (collapsed / expanded).
- Default to collapsed. Ctrl+E (already wired for tool results) extends to tool-call cards.
- Errors auto-expand.
- ~150 LOC.

### Phase 3 — Hierarchy fixes

- Heading colors: monochrome ladder.
- Inline code: drop literal backticks.
- Sidebar tab strip: kill stipple.
- Assistant message: thin left rule `│` in `gray.4` (subtle visual structure).
- ~100 LOC.

### Phase 4 — Chrome correctness

- Add `unicode-width` dep.
- Replace `.len()` byte measurements with `unicode_width::UnicodeWidthStr::width(...)`.
- Share box-width values between top/bottom border rendering.
- ~50 LOC.

### Phase 5 — Empty / loading states

- "Idle" empty chat: subtle hero with `arawn` wordmark + `Press / for commands · Tab to toggle sidebar`.
- "Connecting" overlay during OAuth flow: heartbeat dot every 30s.
- "Generating" indicator: replace heavy `█` cursor with `▌` thin block.
- ~80 LOC.

## Alternatives Considered

### Alternative A — Configurable themes from day one

Ship a theme system from the start (gruvbox, solarized, dracula presets).

- **Pro:** User-friendly.
- **Con:** Premature. Ship one opinionated palette first, learn from it, theme later.

### Alternative B — Adopt Radix Colors / Tailwind palette as a dep

Use a Rust port of an existing color system instead of inventing one.

- **Pro:** Less invention.
- **Con:** Ratatui's `Color` type is constrained (RGB or named ANSI); existing palettes ship as hex strings or web color formats. The conversion is trivial, but pulling a dep for ~30 constants is overkill. Hardcode the 12-step scale as `Rgb(...)` constants in `theme.rs`.

### Alternative C — Hard-RGB everything (ignore terminal palette)

All colors are `Rgb(...)`; named ANSI never used.

- **Pro:** Total control over appearance.
- **Con:** Ignores user's terminal theme. Some colors (e.g. status indicators) read better when they honor the user's palette.

**Pick:** **Mostly Alternative C, with a small named-ANSI escape hatch** for accent colors that should adapt to terminal palette (e.g. `Color::Yellow` for spinner/in-progress, since "yellow" is universally the in-progress signal). The 12-step gray scale + status semantic colors are hard RGB; spinner accent is named ANSI.

## Implementation Plan

Decompose into tasks at design-phase exit. Phase order matches dependency:

1. **Phase 1 — Palette + theme wiring.** Foundational, unlocks everything else. ~200 LOC.
2. **Phase 2 — Tool-call collapsed rendering.** Single most visible change. ~150 LOC.
3. **Phase 3 — Hierarchy fixes.** Headings, inline code, sidebar tab, assistant gutter. ~100 LOC.
4. **Phase 4 — Chrome correctness.** Display-width measurement. ~50 LOC.
5. **Phase 5 — Empty/loading states.** Hero, OAuth heartbeat, thinner cursor. ~80 LOC.

Total: ~580 LOC + snapshot rebaselines.

Estimated complexity: **M**. Lots of small mechanical wins, no architectural risk. Pairs naturally with I-0035 (Personal-assistant identity layer) — when the dashboard surfaces from I-0035 land, they'll already look like a designed product because this initiative shipped first.

## Status Updates

### 2026-05-06 — Filed (discovery)

Initiative filed alongside I-0035 after the same parallel three-agent design review. Visual reviewer surfaced ~20 distinct issues; clustering showed they're all symptoms of one root cause: `theme.rs` exists but no caller imports from it. Phase 1 (wiring `theme.rs` into actual call sites) is the single highest-leverage move and should land first. Subsequent phases are mostly mechanical cleanup once the palette layer is real.

Pairs with I-0035 — both should be in flight in parallel so the personal-assistant dashboard surfaces (I-0035) land into a coherent visual system (this initiative). Without I-0036, I-0035's new widgets would inherit the same color overload + drifting grays.