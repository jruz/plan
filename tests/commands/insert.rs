use std::{fs, path, process};

use crate::utils::binary::{stderr, stdout};

const REFERENCE: &str = include_str!("../../PLAN.example.md");
const FRAGMENT: &str = "## Phase 42: Import me\n\nKeep the words Phase 42 here.\n\n### Specs\n\n- [Spec](spec.md)\n\n### Files\n\n- `src/main.rs`\n\n### Todolist\n\n- [x] Draft the change.\n\n### Acceptance Criteria\n\n- [ ] Test the change.";

fn run(root: &path::Path, arguments: &[&str]) -> process::Output {
    process::Command::new(env!("CARGO_BIN_EXE_plan"))
        .current_dir(root)
        .args(arguments)
        .output()
        .expect("run plan command")
}

fn headings(source: &str) -> Vec<&str> {
    source
        .lines()
        .filter(|line| line.starts_with("## Phase "))
        .collect()
}

#[test]
fn inserts_before_between_and_after_existing_phases() {
    let directory = tempfile::tempdir().expect("create test directory");
    let root = directory.path();
    let file = root.join("PLAN.md");
    for (position, expected) in [
        (
            "1",
            [
                "## Phase 1: Starter",
                "## Phase 2: Explain validation",
                "## Phase 3: Add troubleshooting guidance",
            ],
        ),
        (
            "2",
            [
                "## Phase 1: Explain validation",
                "## Phase 2: Starter",
                "## Phase 3: Add troubleshooting guidance",
            ],
        ),
        (
            "3",
            [
                "## Phase 1: Explain validation",
                "## Phase 2: Add troubleshooting guidance",
                "## Phase 3: Starter",
            ],
        ),
    ] {
        fs::write(&file, REFERENCE).expect("write input plan");
        let preview = run(root, &["insert", position, "Starter", "--dry-run"]);
        let diagnostic = stderr(&preview);
        assert!(preview.status.success(), "{diagnostic}");
        let proposed = stdout(&preview);
        let actual = headings(&proposed);
        assert_eq!(actual, expected);
        let unchanged = fs::read_to_string(&file).expect("read unchanged plan");
        assert_eq!(unchanged, REFERENCE);
        let backups = root.join(".plan");
        assert!(!backups.exists());
    }
}

#[test]
fn imports_a_phase_and_keeps_the_original_backup() {
    let directory = tempfile::tempdir().expect("create test directory");
    let root = directory.path();
    let file = root.join("PLAN.md");
    let fragment = root.join("phase.md");
    fs::write(&file, REFERENCE).expect("write input plan");
    fs::write(&fragment, FRAGMENT).expect("write phase fragment");
    let preview = run(root, &["insert", "2", "--from", "phase.md", "--dry-run"]);
    let diagnostic = stderr(&preview);
    assert!(preview.status.success(), "{diagnostic}");
    let proposed = stdout(&preview);
    let expected_phase = FRAGMENT.replacen("## Phase 42:", "## Phase 2:", 1);
    assert!(proposed.contains(&expected_phase));
    assert!(proposed.contains("## Phase 3: Add troubleshooting guidance"));

    let applied = run(root, &["insert", "2", "--from", "phase.md"]);
    let apply_diagnostic = stderr(&applied);
    assert!(applied.status.success(), "{apply_diagnostic}");
    let actual = fs::read_to_string(&file).expect("read updated plan");
    assert_eq!(actual, proposed);
    let unchanged = fs::read_to_string(fragment).expect("read unchanged fragment");
    assert_eq!(unchanged, FRAGMENT);
    let message = stdout(&applied);
    let backup = message
        .trim_end()
        .strip_prefix("Backup: ")
        .expect("backup path");
    let original = fs::read_to_string(backup).expect("read backup");
    assert_eq!(original, REFERENCE);
    let checked = run(root, &["check"]);
    let check_diagnostic = stderr(&checked);
    assert!(checked.status.success(), "{check_diagnostic}");
}

#[test]
fn rejects_invalid_positions_and_inputs_without_writing() {
    let directory = tempfile::tempdir().expect("create test directory");
    let root = directory.path();
    let file = root.join("PLAN.md");
    let fragment = root.join("phase.md");
    fs::write(&file, REFERENCE).expect("write input plan");
    let extra = format!("{FRAGMENT}\n\n---\n\n{FRAGMENT}\n");
    fs::write(fragment, extra).expect("write multiple phases");
    let cases: &[&[&str]] = &[
        &["insert", "0", "Starter"],
        &["insert", "4", "Starter"],
        &["insert", "1"],
        &["insert", "1", "line\nbreak"],
        &["insert", "2", "--from", "phase.md"],
        &["insert", "2", "Starter", "--from", "phase.md"],
        &["insert", "2", "--from", "missing.md"],
    ];
    for args in cases {
        let output = run(root, args);
        assert!(!output.status.success(), "unexpected success: {args:?}");
        let unchanged = fs::read_to_string(&file).expect("read unchanged plan");
        assert_eq!(unchanged, REFERENCE);
        let backups = root.join(".plan");
        assert!(!backups.exists());
    }
}
