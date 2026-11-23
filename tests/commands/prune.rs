use std::{fs, path, process};

use crate::utils::binary::{stderr, stdout};

const REFERENCE: &str = include_str!("../../PLAN.example.md");

fn run(directory: &path::Path, arguments: &[&str]) -> process::Output {
    process::Command::new(env!("CARGO_BIN_EXE_plan"))
        .current_dir(directory)
        .args(arguments)
        .output()
        .expect("run plan command")
}

fn assert_prune(source: &str, expected: &str, success: bool) {
    let directory = tempfile::tempdir().expect("create test directory");
    let root = directory.path();
    let file = root.join("PLAN.md");
    fs::write(&file, source).expect("write plan");
    let output = run(root, &["prune"]);
    let diagnostic = stderr(&output);
    assert_eq!(output.status.success(), success, "{diagnostic}");
    let actual = fs::read_to_string(&file).expect("read plan");
    assert_eq!(actual, expected);
    if source == expected {
        let backups = root.join(".plan");
        assert!(!backups.exists());
    }
    if success {
        let checked = run(root, &["check"]);
        let check_diagnostic = stderr(&checked);
        assert!(checked.status.success(), "{check_diagnostic}");
    }
}

#[test]
fn removes_completed_phases_without_touching_other_text() {
    let (intro, rest) = REFERENCE.split_once("## Phase 1:").expect("first phase");
    let (_, open) = rest.split_once("## Phase 2:").expect("second phase");
    let source = REFERENCE.replace("Show how to fix", "See Phase 2 for how to fix");
    let open = open.replace("Show how to fix", "See Phase 2 for how to fix");
    let expected = format!("{intro}## Phase 1:{open}");
    assert_prune(&source, &expected, true);
    let crlf_source = source.replace('\n', "\r\n");
    let crlf_expected = expected.replace('\n', "\r\n");
    assert_prune(&crlf_source, &crlf_expected, true);
}

#[test]
fn keeps_phases_with_any_open_item() {
    let open_criterion =
        REFERENCE.replace("- [x] `just test` passes.", "- [ ] `just test` passes.");
    assert_prune(&open_criterion, &open_criterion, true);
    let open_todo = REFERENCE.replace(
        "- [x] Explain where diagnostics",
        "- [ ] Explain where diagnostics",
    );
    assert_prune(&open_todo, &open_todo, true);
}

#[test]
fn handles_all_phases_complete_or_invalid() {
    let all_complete = REFERENCE.replace("- [ ] ", "- [x] ");
    let (intro, _) = REFERENCE.split_once("## Phase 1:").expect("first phase");
    let expected = format!("{intro}## Blocker\n");
    assert_prune(&all_complete, &expected, true);
    assert_prune(&expected, &expected, true);
    let invalid = REFERENCE.replace("## Phase 2:", "## Phase 3:");
    assert_prune(&invalid, &invalid, false);
}

#[test]
fn dry_run_prints_the_same_plan_that_apply_writes() {
    let directory = tempfile::tempdir().expect("create test directory");
    let root = directory.path();
    let file = root.join("PLAN.md");
    let source = REFERENCE.replace('\n', "\r\n");
    fs::write(&file, &source).expect("write plan");
    let preview = run(root, &["prune", "--dry-run"]);
    let diagnostic = stderr(&preview);
    assert!(preview.status.success(), "{diagnostic}");
    let proposed = stdout(&preview);
    assert!(proposed.contains("## Phase 1: Add troubleshooting guidance"));
    assert!(!proposed.contains("## Phase 1: Explain validation"));
    let unchanged = fs::read_to_string(&file).expect("read original");
    assert_eq!(unchanged, source);
    let backups = root.join(".plan");
    assert!(!backups.exists());

    let first = apply_and_read_backup(root, &source);
    let actual = fs::read_to_string(&file).expect("read updated plan");
    assert_eq!(actual, proposed);
    let ignore = backups.join(".gitignore");
    let rules = fs::read_to_string(ignore).expect("read ignore rules");
    assert_eq!(rules, "*\n");

    let second_source = REFERENCE.replace("# Plan:", "# Plan: Updated");
    fs::write(&file, &second_source).expect("write second plan");
    let second = apply_and_read_backup(root, &second_source);
    assert_ne!(first, second);
    let first_contents = fs::read_to_string(first).expect("read first snapshot again");
    assert_eq!(first_contents, source);
}

fn apply_and_read_backup(root: &path::Path, source: &str) -> path::PathBuf {
    let output = run(root, &["prune"]);
    let diagnostic = stderr(&output);
    assert!(output.status.success(), "{diagnostic}");
    let message = stdout(&output);
    let location = message
        .trim_end()
        .strip_prefix("Backup: ")
        .expect("backup path");
    let path = path::PathBuf::from(location);
    let root = root.canonicalize().expect("canonical root");
    let backups = root.join(".plan");
    assert_eq!(path.parent(), Some(backups.as_path()));
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .expect("snapshot name");
    let stem = name
        .strip_prefix("PLAN-")
        .and_then(|name| name.strip_suffix(".md"))
        .expect("snapshot filename format");
    let timestamp = stem.get(..20).expect("timestamp prefix");
    assert!(timestamp.contains('T'));
    assert!(timestamp.ends_with('Z'));
    let snapshot = fs::read_to_string(&path).expect("read snapshot");
    assert_eq!(snapshot, source);
    path
}

#[test]
fn backup_failure_leaves_the_plan_untouched() {
    let directory = tempfile::tempdir().expect("create test directory");
    let root = directory.path();
    let file = root.join("PLAN.md");
    let backups = root.join(".plan");
    fs::write(&file, REFERENCE).expect("write plan");
    fs::write(backups, "not a directory").expect("block backup directory");
    let output = run(root, &["prune"]);
    assert!(!output.status.success());
    let diagnostic = stderr(&output);
    assert!(
        diagnostic.contains("could not replace plan"),
        "{diagnostic}"
    );
    let unchanged = fs::read_to_string(&file).expect("read unchanged plan");
    assert_eq!(unchanged, REFERENCE);
}
