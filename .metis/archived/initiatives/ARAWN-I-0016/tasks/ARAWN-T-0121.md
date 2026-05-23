---
id: workflow-authoring-scaffold-agent
level: task
title: "Workflow authoring scaffold — agent generates, compiles, and installs .cloacina packages"
short_code: "ARAWN-T-0121"
created_at: 2026-04-07T21:38:38.676659+00:00
updated_at: 2026-04-09T12:54:11.102931+00:00
parent: ARAWN-I-0016
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0016
---

# Workflow authoring scaffold — agent generates, compiles, and installs .cloacina packages

## Objective

Enable the agent to author workflow packages during a conversation. The agent generates a Cargo project using `cloacina-workflow` macros (`#[workflow]`, `#[task]`, `#[trigger]`), compiles it to a cdylib, packages it with `cloacina-ctl`, and installs it to the watched directory. The reconciler hot-loads it automatically.

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [ ] Bug - Production issue that needs fixing
- [ ] Feature - New functionality or enhancement  
- [ ] Tech Debt - Code improvement or refactoring
- [ ] Chore - Maintenance or setup work

### Priority
- [ ] P0 - Critical (blocks users/revenue)
- [ ] P1 - High (important for user experience)
- [ ] P2 - Medium (nice to have)
- [ ] P3 - Low (when time permits)

### Impact Assessment **[CONDITIONAL: Bug]**
- **Affected Users**: {Number/percentage of users affected}
- **Reproduction Steps**: 
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected vs Actual**: {What should happen vs what happens}

### Business Justification **[CONDITIONAL: Feature]**
- **User Value**: {Why users need this}
- **Business Value**: {Impact on metrics/revenue}
- **Effort Estimate**: {Rough size - S/M/L/XL}

### Technical Debt Impact **[CONDITIONAL: Tech Debt]**
- **Current Problems**: {What's difficult/slow/buggy now}
- **Benefits of Fixing**: {What improves after refactoring}
- **Risk Assessment**: {Risks of not addressing this}

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Scaffold template in `arawn-workflow`: generates a valid Cargo project with `cloacina-workflow = { version = "0.4", features = ["packaged"] }`, `crate-type = ["cdylib", "rlib"]`, and a minimal `#[workflow]` + `#[task]` in `src/lib.rs`
- [ ] Agent can generate the scaffold via shell tool: `mkdir /tmp/my-workflow && write files && cargo build --release`
- [ ] `cloacina-ctl package . -o ~/.arawn/workflows/my-workflow.cloacina` produces a valid package
- [ ] Reconciler detects the new `.cloacina` file and loads it (verify via runner logs)
- [ ] Cron trigger defined via `#[trigger(on = "...", cron = "...")]` is registered and fires on schedule
- [ ] End-to-end test: agent authors a workflow with 2 data tasks and a cron trigger, package is installed, reconciler loads it, manual execution succeeds

### Key files
- `crates/arawn-workflow/src/scaffold.rs` — template generation (Cargo.toml, src/lib.rs, package.toml)
- Agent uses existing `shell` tool for `cargo build` and `cloacina-ctl package`

### Dependencies
- T-0120 (runner must be embedded and running)
- `cloacina-ctl` must be installed on the system (or bundled)

## Test Cases **[CONDITIONAL: Testing Task]**

{Delete unless this is a testing task}

### Test Case 1: {Test Case Name}
- **Test ID**: TC-001
- **Preconditions**: {What must be true before testing}
- **Steps**: 
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

### Test Case 2: {Test Case Name}
- **Test ID**: TC-002
- **Preconditions**: {What must be true before testing}
- **Steps**: 
  1. {Step 1}
  2. {Step 2}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

## Documentation Sections **[CONDITIONAL: Documentation Task]**

{Delete unless this is a documentation task}

### User Guide Content
- **Feature Description**: {What this feature does and why it's useful}
- **Prerequisites**: {What users need before using this feature}
- **Step-by-Step Instructions**:
  1. {Step 1 with screenshots/examples}
  2. {Step 2 with screenshots/examples}
  3. {Step 3 with screenshots/examples}

### Troubleshooting Guide
- **Common Issue 1**: {Problem description and solution}
- **Common Issue 2**: {Problem description and solution}
- **Error Messages**: {List of error messages and what they mean}

### API Documentation **[CONDITIONAL: API Documentation]**
- **Endpoint**: {API endpoint description}
- **Parameters**: {Required and optional parameters}
- **Example Request**: {Code example}
- **Example Response**: {Expected response format}

## Implementation Notes **[CONDITIONAL: Technical Task]**

{Keep for technical tasks, delete for non-technical. Technical details, approach, or important considerations}

### Technical Approach
{How this will be implemented}

### Dependencies
{Other tasks or systems this depends on}

### Risk Considerations
{Technical risks and mitigation strategies}

## Status Updates **[REQUIRED]**

*To be added during implementation*