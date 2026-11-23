---
iteration: 1
max_iterations: 20
---

# Plan: Document the CLI

- Show new users how to validate their first plan.
- Mark each todo complete immediately after finishing it.
- Review: after each phase.
- Commits: after each phase.

## Specs

- [CLI reference](README.md)

---

## Phase 1: Explain validation

Document how to run the checker and read its diagnostics.

### Specs

- [Check command](README.md#check-a-plan)

### Files

- `README.md`

### Todolist

- [x] Document the `plan check` command.
- [x] Explain where diagnostics appear.

### Acceptance Criteria

- [x] `plan check` accepts a valid plan.
- [x] `just check` passes.
- [x] `just test` passes.

---

## Phase 2: Add troubleshooting guidance

Show how to fix an out-of-order phase number.

### Specs

- [Check command](README.md#check-a-plan)

### Files

- `README.md`

### Todolist

- [ ] Show a diagnostic for an out-of-order phase.
- [ ] Show the corrected phase heading.

### Acceptance Criteria

- [ ] `plan check` rejects the broken example at the expected line.
- [ ] `plan check` accepts the corrected example.
- [ ] `just check` passes.
- [ ] `just test` passes.

---

## Blocker
