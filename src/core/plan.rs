use std::ops;

use pulldown_cmark::HeadingLevel;

use super::{frontmatter, markdown};

pub(in crate::core) const PHASE_PREFIX: &str = "## Phase ";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(" invalid phase source range")]
    EditRange,
    #[error(" invalid phase fragment at line {0}")]
    Fragment(#[source] Box<Self>),
    #[error(" expected exactly one phase in the import file")]
    FragmentCount,
    #[error(" import file must start with `## Phase N: <title>`")]
    FragmentHeading,
    #[error(transparent)]
    Markdown(#[from] markdown::Error),
    #[error(" insertion position {position} is out of range (expected 1 through {last})")]
    Position { last: usize, position: usize },
    #[error("{line}: {message}")]
    Structure { line: usize, message: String },
    #[error(" phase title must be a single non-empty line")]
    Title,
}

pub(in crate::core) struct Violation {
    message: String,
    offset: usize,
}

pub(in crate::core) struct Sections<'blocks, 'source> {
    pub end: usize,
    pub remaining: &'blocks [markdown::Block<'source>],
}

pub(in crate::core) struct Plan {
    pub blocker: usize,
    pub phases: Vec<Phase>,
}

pub(in crate::core) struct Phase {
    pub number: ops::Range<usize>,
    pub start: usize,
    pub status: Status,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::core) enum Status {
    Complete,
    Open,
}

pub fn check(source: &str) -> Result<(), Error> {
    parse(source)?;
    Ok(())
}

pub fn prune(source: &str) -> Result<Option<String>, Error> {
    let plan = parse(source)?;
    if !plan
        .phases
        .iter()
        .any(|phase| phase.status == Status::Complete)
    {
        return Ok(None);
    }
    let length = source.len();
    let mut updated = String::with_capacity(length);
    let mut cursor = 0;
    let mut number: usize = 1;
    let mut phases = plan.phases.iter().peekable();
    while let Some(phase) = phases.next() {
        let start = line_start(source, phase.start)?;
        append(&mut updated, source, cursor..start)?;
        let end = match phases.peek() {
            Some(next) => line_start(source, next.start)?,
            None => line_start(source, plan.blocker)?,
        };
        if phase.status == Status::Open {
            append(&mut updated, source, start..phase.number.start)?;
            let digits = number.to_string();
            updated.push_str(&digits);
            append(&mut updated, source, phase.number.end..end)?;
            number += 1;
        }
        cursor = end;
    }
    append(&mut updated, source, cursor..length)?;
    check(&updated)?;
    let replacement = Some(updated);
    Ok(replacement)
}

pub(in crate::core) fn line_start(source: &str, offset: usize) -> Result<usize, Error> {
    let prefix = source.get(..offset).ok_or(Error::EditRange)?;
    match prefix.rfind('\n') {
        Some(newline) => Ok(newline + 1),
        None => Ok(0),
    }
}

pub(in crate::core) fn append(
    output: &mut String,
    source: &str,
    range: ops::Range<usize>,
) -> Result<(), Error> {
    let text = source.get(range).ok_or(Error::EditRange)?;
    output.push_str(text);
    Ok(())
}

pub(in crate::core) fn parse(source: &str) -> Result<Plan, Error> {
    let body = match frontmatter::body(source) {
        Ok(offset) => offset,
        Err(issue) => {
            let (offset, error) = issue;
            let message = error.to_string();
            let violation = Violation { message, offset };
            let diagnostic = at_line(source, violation);
            return Err(diagnostic);
        }
    };
    let blocks = markdown::blocks(source, body)?;
    let mut sections = Sections {
        remaining: &blocks,
        end: source.len(),
    };
    match validate(&mut sections) {
        Ok(plan) => Ok(plan),
        Err(issue) => {
            let diagnostic = at_line(source, issue);
            Err(diagnostic)
        }
    }
}

pub(in crate::core) fn at_line(source: &str, issue: Violation) -> Error {
    let line = source
        .bytes()
        .take(issue.offset)
        .filter(|byte| *byte == b'\n')
        .count()
        + 1;
    Error::Structure {
        line,
        message: issue.message,
    }
}

fn invalid<T>(offset: usize, message: &str) -> Result<T, Violation> {
    let message = message.to_string();
    Err(Violation { message, offset })
}

fn take<'blocks, 'source>(
    sections: &mut Sections<'blocks, 'source>,
    message: &str,
    accepts: impl FnOnce(&markdown::Block<'_>) -> bool,
) -> Result<&'blocks markdown::Block<'source>, Violation> {
    let Some((block, rest)) = sections.remaining.split_first() else {
        return invalid(sections.end, message);
    };
    if !accepts(block) {
        return invalid(block.offset, message);
    }
    sections.remaining = rest;
    Ok(block)
}

fn heading(block: &markdown::Block<'_>, level: HeadingLevel, text: &str) -> bool {
    matches!(block.kind, markdown::Kind::Heading(actual) if actual == level)
        && block.text.trim_end() == text
}

fn section(
    sections: &mut Sections<'_, '_>,
    level: HeadingLevel,
    text: &str,
) -> Result<(), Violation> {
    let message = format!("expected `{text}`");
    take(sections, &message, |block| heading(block, level, text))?;
    Ok(())
}

pub(in crate::core) fn separator(sections: &mut Sections<'_, '_>) -> Result<(), Violation> {
    take(sections, "expected `---` section separator", |block| {
        matches!(block.kind, markdown::Kind::Rule) && block.text.trim_end() == "---"
    })?;
    Ok(())
}

fn optional_list(
    sections: &mut Sections<'_, '_>,
    message: &str,
    accepts: impl Fn(&markdown::Item<'_>) -> bool,
) -> Result<(), Violation> {
    if let Some((block, rest)) = sections.remaining.split_first()
        && let markdown::Kind::List(items) = &block.kind
    {
        if let Some(item) = items.iter().find(|item| !accepts(item)) {
            return invalid(item.offset, message);
        }
        sections.remaining = rest;
    }
    Ok(())
}

fn checklist(sections: &mut Sections<'_, '_>, name: &str) -> Result<Status, Violation> {
    section(sections, HeadingLevel::H3, name)?;
    let message = "expected a checklist with `- [ ]` or `- [x]` items";
    let block = take(sections, message, |block| {
        matches!(block.kind, markdown::Kind::List(_))
    })?;
    let markdown::Kind::List(items) = &block.kind else {
        return invalid(block.offset, message);
    };
    let mut status = Status::Complete;
    for item in items {
        let prefix = match item.checked {
            Some(true) => "- [x] ",
            Some(false) => {
                status = Status::Open;
                "- [ ] "
            }
            None => return invalid(item.offset, message),
        };
        if item
            .text
            .trim_end()
            .strip_prefix(prefix)
            .is_none_or(|text| text.trim().is_empty() || text.contains('\n'))
        {
            return invalid(item.offset, message);
        }
    }
    Ok(status)
}

fn validate(sections: &mut Sections<'_, '_>) -> Result<Plan, Violation> {
    take(sections, "expected `# Plan: <title>`", |block| {
        matches!(block.kind, markdown::Kind::Heading(HeadingLevel::H1))
            && block
                .text
                .trim_end()
                .strip_prefix("# Plan: ")
                .is_some_and(|title| !title.trim().is_empty())
    })?;
    take(sections, "expected an introductory bullet list", |block| {
        matches!(block.kind, markdown::Kind::List(_))
    })?;
    section(sections, HeadingLevel::H2, "## Specs")?;
    optional_list(sections, "expected a Markdown spec link", |item| {
        item.has_link
    })?;
    separator(sections)?;

    let mut phases = Vec::new();
    while let Some(block) = sections.remaining.first() {
        if heading(block, HeadingLevel::H2, "## Blocker") {
            break;
        }
        let number = phases.len() + 1;
        let phase = phase(sections, number)?;
        separator(sections)?;
        phases.push(phase);
    }
    let blocker = take(sections, "expected `## Blocker`", |block| {
        heading(block, HeadingLevel::H2, "## Blocker")
    })?;
    if let Some(block) = sections.remaining.iter().find(|block| {
        matches!(
            block.kind,
            markdown::Kind::Heading(HeadingLevel::H1 | HeadingLevel::H2)
        )
    }) {
        return invalid(
            block.offset,
            "`## Blocker` must be the last top-level section",
        );
    }
    Ok(Plan {
        blocker: blocker.offset,
        phases,
    })
}

pub(in crate::core) fn phase(
    sections: &mut Sections<'_, '_>,
    number: usize,
) -> Result<Phase, Violation> {
    let digits = number.to_string();
    let prefix = format!("{PHASE_PREFIX}{digits}: ");
    let message = format!("expected `{PHASE_PREFIX}{digits}: <title>` or `## Blocker`");
    let prefix = prefix.as_str();
    let heading = take(sections, &message, |block| {
        matches!(block.kind, markdown::Kind::Heading(HeadingLevel::H2))
            && block
                .text
                .trim_end()
                .strip_prefix(prefix)
                .is_some_and(|title| !title.trim().is_empty())
    })?;
    take(sections, "expected a one-line phase description", |block| {
        matches!(block.kind, markdown::Kind::Paragraph) && !block.text.trim_end().contains('\n')
    })?;
    section(sections, HeadingLevel::H3, "### Specs")?;
    optional_list(sections, "expected a Markdown spec link", |item| {
        item.has_link
    })?;
    section(sections, HeadingLevel::H3, "### Files")?;
    optional_list(
        sections,
        "expected one backticked file path per bullet",
        |item| {
            item.text
                .trim_end()
                .strip_prefix("- `")
                .and_then(|text| text.strip_suffix('`'))
                .is_some_and(|path| !path.trim().is_empty() && !path.contains(['`', '\n']))
        },
    )?;
    let todos = checklist(sections, "### Todolist")?;
    let criteria = checklist(sections, "### Acceptance Criteria")?;
    let status = match (todos, criteria) {
        (Status::Complete, Status::Complete) => Status::Complete,
        (Status::Open, _) | (_, Status::Open) => Status::Open,
    };
    let start = heading.offset;
    let digits_start = start + PHASE_PREFIX.len();
    let digits_end = digits_start + digits.len();
    Ok(Phase {
        number: digits_start..digits_end,
        start,
        status,
    })
}
