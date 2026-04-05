an---
id: session-management-ux-resume
level: initiative
title: "Session management UX — resume, branch, export, compact, context visualization"
short_code: "ARAWN-I-0011"
created_at: 2026-04-03T02:07:46.172744+00:00
updated_at: 2026-04-03T02:07:46.172744+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: M
initiative_id: session-management-ux-resume
---

# Session management UX — resume, branch, export, compact, context visualization Initiative

*This template includes sections for various types of initiatives. Delete sections that don't apply to your specific use case.*

## Context

Arawn has basic session persistence (JSONL + SQLite) and `--session` / `--list-sessions` CLI flags, but no rich session management UX. Claude Code has resume with interactive picker, conversation branching, export, rename, manual compact, clear, context visualization, and cost tracking — all as slash commands or TUI features. We need to evaluate each of these, decide what fits arawn's model, and build the ones that matter.

## Goals & Non-Goals

**Goals:**
- Evaluate each Claude Code session feature for fit in arawn
- Design and implement the high-value ones
- Establish a slash-command or TUI-command pattern for user-initiated actions

**Non-Goals:**
- Remote session transfer (teleport)
- Session sync to cloud

## Requirements **[CONDITIONAL: Requirements-Heavy Initiative]**

{Delete if not a requirements-focused initiative}

### User Requirements
- **User Characteristics**: {Technical background, experience level, etc.}
- **System Functionality**: {What users expect the system to do}
- **User Interfaces**: {How users will interact with the system}

### System Requirements
- **Functional Requirements**: {What the system should do - use unique identifiers}
  - REQ-001: {Functional requirement 1}
  - REQ-002: {Functional requirement 2}
- **Non-Functional Requirements**: {How the system should behave}
  - NFR-001: {Performance requirement}
  - NFR-002: {Security requirement}

## Use Cases **[CONDITIONAL: User-Facing Initiative]**

{Delete if not user-facing}

### Use Case 1: {Use Case Name}
- **Actor**: {Who performs this action}
- **Scenario**: {Step-by-step interaction}
- **Expected Outcome**: {What should happen}

### Use Case 2: {Use Case Name}
- **Actor**: {Who performs this action}
- **Scenario**: {Step-by-step interaction}
- **Expected Outcome**: {What should happen}

## Architecture **[CONDITIONAL: Technically Complex Initiative]**

{Delete if not technically complex}

### Overview
{High-level architectural approach}

### Component Diagrams
{Describe or link to component diagrams}

### Class Diagrams
{Describe or link to class diagrams - for OOP systems}

### Sequence Diagrams
{Describe or link to sequence diagrams - for interaction flows}

### Deployment Diagrams
{Describe or link to deployment diagrams - for infrastructure}

## Detailed Design

### Feature Evaluation (to be decided during discovery)

Each feature needs a decision: **build**, **defer**, or **skip**.

| Feature | Claude Code | What it does | Arawn status | Decision |
|---------|------------|-------------|-------------|----------|
| **Resume** | `/resume` | Interactive picker to resume previous session by name/UUID | Have `--session` flag, no picker | TBD |
| **Branch** | `/branch` | Fork conversation at current point, preserving history | Nothing | TBD |
| **Export** | `/export` | Export conversation to plain text/markdown file | Nothing | TBD |
| **Rename** | `/rename` | Set human-readable session name (auto-generate or manual) | Nothing | TBD |
| **Compact** | `/compact` | Manual full-conversation summarization with optional focus | Have auto-compact, no manual trigger | TBD |
| **Clear** | `/clear` | Wipe conversation history, fresh start same session | Nothing | TBD |
| **Context** | `/context` | Colored grid showing token usage per message segment | Nothing | TBD |
| **Cost** | `/cost` | Show accumulated USD cost, token counts, duration | Have token stats in CLI output | TBD |
| **Copy** | `/copy` | Copy last assistant response to clipboard | Nothing | TBD |
| **Doctor** | `/doctor` | Diagnose installation, config, dependencies | Nothing | TBD |

### Discovery Questions
- Do we implement these as slash commands (typed in TUI input), CLI flags, or both?
- What's the minimum set needed for a good session workflow? (probably: resume, compact, clear, cost)
- Does branching have value for a personal assistant or is it more of a dev-tool feature?
- Context visualization — is the token grid useful or is a simple "X/Y tokens used" sufficient?

### Implementation Pattern
Once decisions are made, each becomes a task under this initiative. The slash-command infrastructure itself may be the first task — establishing how user commands are parsed, dispatched, and rendered in the TUI.

## UI/UX Design **[CONDITIONAL: Frontend Initiative]**

{Delete if no UI components}

### User Interface Mockups
{Describe or link to UI mockups}

### User Flows
{Describe key user interaction flows}

### Design System Integration
{How this fits with existing design patterns}

## Testing Strategy **[CONDITIONAL: Separate Testing Initiative]**

{Delete if covered by separate testing initiative}

### Unit Testing
- **Strategy**: {Approach to unit testing}
- **Coverage Target**: {Expected coverage percentage}
- **Tools**: {Testing frameworks and tools}

### Integration Testing
- **Strategy**: {Approach to integration testing}
- **Test Environment**: {Where integration tests run}
- **Data Management**: {Test data strategy}

### System Testing
- **Strategy**: {End-to-end testing approach}
- **User Acceptance**: {How UAT will be conducted}
- **Performance Testing**: {Load and stress testing}

### Test Selection
{Criteria for determining what to test}

### Bug Tracking
{How defects will be managed and prioritized}

## Alternatives Considered **[REQUIRED]**

{Alternative approaches and why they were rejected}

## Implementation Plan **[REQUIRED]**

{Phases and timeline for execution}