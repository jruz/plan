# plan

CLI for operating on a `PLAN.md` file.

## Check a plan

Use [PLAN.example.md](PLAN.example.md) as a reference for your own `PLAN.md`.

```sh
plan check
```

`plan check` reads `PLAN.md` in the current directory without changing it. It exits silently on success. On the first structural error it prints `PLAN.md:line: message` to stderr and exits with status 1.

The checker requires:

- Frontmatter delimited by `---`, with one non-negative `iteration` and one positive `max_iterations`.
- A `# Plan: <title>` heading, an introductory bullet list, and a `## Specs` section.
- Sequential `## Phase N: <title>` headings starting at 1.
- A one-line phase description followed by `### Specs`, `### Files`, `### Todolist`, and `### Acceptance Criteria`, in that order.
- Markdown links in non-empty spec lists.
- One backticked path per file bullet.
- Non-empty todo and acceptance checklists using single-line `- [ ]` or `- [x]` items.
- `---` separators after the plan-level specs and after each phase.
- A final `## Blocker` section, even if the plan has no phases.

## Remove completed phases

```sh
plan prune --dry-run
plan prune
```

`plan prune` removes a phase only when every item in both its Todolist and Acceptance Criteria is checked. It renumbers the remaining phase headings starting at 1 without changing references in prose. Invalid plans and plans with nothing to prune are left untouched.

`--dry-run` prints the proposed `PLAN.md` without writing it. A real edit saves the original in `.plan/` with a UTC timestamp and prints the backup path. The CLI creates `.plan/.gitignore` to keep snapshots out of Git.

To restore, copy the saved snapshot over `PLAN.md`. If the backup cannot be saved, the CLI leaves the plan untouched.

## Licence

Licensed under the [EUPL 1.2](LICENSE).
