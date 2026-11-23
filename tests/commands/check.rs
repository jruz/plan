use std::{env, fs, process};

use crate::utils::binary::{stderr, stdout};

const PLAN: &str = "---\n\
iteration: 1\n\
max_iterations: 20\n\
---\n\n\
# Plan: Caf\u{e9} release\n\n\
- Ship the release.\n\n\
## Specs\n\n\
- [Release](release.md)\n\n\
---\n\n\
## Phase 1: Build\n\n\
Build the package.\n\n\
### Specs\n\n\
### Files\n\n\
- `src/main.rs`\n\n\
### Todolist\n\n\
- [ ] Compile.\n\n\
### Acceptance Criteria\n\n\
- [x] Run the tests.\n\n\
---\n\n\
## Blocker\n\n\
```markdown\n\
## Phase 99: Example only\n\
---\n\
```\n";

fn check(directory: &std::path::Path) -> process::Output {
    process::Command::new(env!("CARGO_BIN_EXE_plan"))
        .current_dir(directory)
        .arg("check")
        .output()
        .expect("run plan check")
}

#[test]
fn checks_valid_and_invalid_plans_without_writing() {
    let temp = env::temp_dir();
    let pid = process::id();
    let name = format!("plan-check-{pid}");
    let directory = temp.join(name);
    fs::create_dir_all(&directory).expect("create fixture directory");
    let file = directory.join("PLAN.md");
    let reference = include_str!("../../PLAN.example.md");
    for source in [
        PLAN.to_string(),
        PLAN.replace('\n', "\r\n"),
        reference.to_string(),
    ] {
        fs::write(&file, &source).expect("write fixture");
        let output = check(&directory);
        let diagnostic = stderr(&output);
        assert!(output.status.success(), "{diagnostic}");
        assert_eq!(stdout(&output), "");
        let unchanged = fs::read_to_string(&file).expect("read fixture");
        assert_eq!(unchanged, source);
    }
    let cases = [
        (
            "iteration: 1",
            "iteration: nope",
            "2: expected a whole number",
        ),
        ("## Phase 1:", "## Phase 3:", "16: expected `## Phase 1:"),
        ("### Files", "### Paths", "22: expected `### Files`"),
        (
            "- [ ] Compile.",
            "- [X] Compile.",
            "28: expected a checklist",
        ),
        (
            "- [x] Run the tests.\n\n---",
            "- [x] Run the tests.",
            "34: expected `---`",
        ),
        ("## Blocker", "## Missing", "36: expected `## Phase 2:"),
    ];
    for (original, replacement, expected) in cases {
        let source = PLAN.replace(original, replacement);
        fs::write(&file, &source).expect("write invalid fixture");
        let output = check(&directory);
        let code: i32 = 1;
        assert_eq!(output.status.code(), Some(code));
        let diagnostic = stderr(&output);
        let prefix = format!("PLAN.md:{expected}");
        assert!(diagnostic.starts_with(&prefix), "{diagnostic}");
        let unchanged = fs::read_to_string(&file).expect("read invalid fixture");
        assert_eq!(unchanged, source);
    }
    fs::remove_file(file).expect("remove test file");
    let output = check(&directory);
    let diagnostic = stderr(&output);
    assert!(!output.status.success());
    assert!(
        diagnostic.contains("PLAN.md: could not read plan"),
        "{diagnostic}"
    );
    fs::remove_dir(directory).expect("remove fixture directory");
}
