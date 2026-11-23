use std::{fs, path, process};

use crate::utils::binary::{stderr, stdout};

const REFERENCE: &str = include_str!("../../PLAN.example.md");
const FIRST: &str = "## Phase 1: Explain validation";
const SECOND: &str = "## Phase 2: Add troubleshooting guidance";
const THIRD: &str = "## Phase 3: Review Phase 1";

fn run(root: &path::Path, arguments: &[&str]) -> process::Output {
    process::Command::new(env!("CARGO_BIN_EXE_plan"))
        .current_dir(root)
        .args(arguments)
        .output()
        .expect("run plan command")
}

#[test]
fn moves_in_both_directions_and_preserves_the_original_snapshot() {
    let directory = tempfile::tempdir().expect("create test directory");
    let root = directory.path();
    let file = root.join("PLAN.md");
    let (prefix, phases) = REFERENCE.split_once(FIRST).expect("first phase");
    let (first_body, rest) = phases.split_once(SECOND).expect("second phase");
    let (second_body, _) = rest.split_once("## Blocker").expect("blocker");
    let third_body = second_body.replace("out-of-order phase number", "reference to Phase 1");
    let blocker = "## Blocker\n\nKeep this note about Phase 1.";
    let original =
        format!("{prefix}{FIRST}{first_body}{SECOND}{second_body}{THIRD}{third_body}{blocker}");
    let original = original.replace('\n', "\r\n");
    fs::write(&file, &original).expect("write input plan");

    let expected = format!(
        "{prefix}## Phase 1: Add troubleshooting guidance{second_body}## Phase 2: Review Phase 1{third_body}## Phase 3: Explain validation{first_body}{blocker}"
    );
    let expected = expected.replace('\n', "\r\n");
    let preview = run(root, &["move", "1", "3", "--dry-run"]);
    assert!(preview.status.success(), "{}", stderr(&preview));
    assert_eq!(stdout(&preview), expected);
    assert_eq!(fs::read_to_string(&file).expect("read plan"), original);
    assert!(!root.join(".plan").exists());

    let applied = run(root, &["move", "1", "3"]);
    assert!(applied.status.success(), "{}", stderr(&applied));
    assert_eq!(
        fs::read_to_string(&file).expect("read moved plan"),
        expected
    );
    let message = stdout(&applied);
    let backup = message
        .trim_end()
        .strip_prefix("Backup: ")
        .expect("backup path");
    assert_eq!(fs::read_to_string(backup).expect("read backup"), original);

    let reversed = run(root, &["move", "3", "1"]);
    assert!(reversed.status.success(), "{}", stderr(&reversed));
    assert_eq!(
        fs::read_to_string(&file).expect("read restored plan"),
        original
    );
    let checked = run(root, &["check"]);
    assert!(checked.status.success(), "{}", stderr(&checked));
}

#[test]
fn rejects_invalid_positions_and_invalid_plans_without_writing() {
    let directory = tempfile::tempdir().expect("create test directory");
    let root = directory.path();
    let file = root.join("PLAN.md");
    let malformed = REFERENCE.replace(SECOND, "## Phase 3: Skipped a position");
    let cases = [
        (REFERENCE, "0", "1", "invalid value"),
        (REFERENCE, "1", "0", "invalid value"),
        (REFERENCE, "3", "1", "source position 3 is out of range"),
        (
            REFERENCE,
            "1",
            "3",
            "destination position 3 is out of range",
        ),
        (malformed.as_str(), "1", "2", "expected `## Phase 2:"),
    ];
    for (source, from, to, diagnostic) in cases {
        fs::write(&file, source).expect("write input plan");
        let output = run(root, &["move", from, to]);
        assert!(!output.status.success());
        assert!(stderr(&output).contains(diagnostic), "{}", stderr(&output));
        assert_eq!(
            fs::read_to_string(&file).expect("read unchanged plan"),
            source
        );
        assert!(!root.join(".plan").exists());
    }
}

#[test]
fn leaves_a_phase_at_its_current_position_without_writing() {
    let directory = tempfile::tempdir().expect("create test directory");
    let root = directory.path();
    let file = root.join("PLAN.md");
    fs::write(&file, REFERENCE).expect("write input plan");

    for args in [&["move", "2", "2"][..], &["move", "2", "2", "--dry-run"]] {
        let output = run(root, args);
        assert!(output.status.success(), "{}", stderr(&output));
        assert_eq!(stdout(&output), "");
        assert_eq!(
            fs::read_to_string(&file).expect("read unchanged plan"),
            REFERENCE
        );
        assert!(!root.join(".plan").exists());
    }
}
