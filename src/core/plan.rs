use pulldown_cmark::HeadingLevel;

use super::{frontmatter, markdown};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Markdown(#[from] markdown::Error),
    #[error("{line}: {message}")]
    Structure { line: usize, message: String },
}

struct Violation {
    message: String,
    offset: usize,
}

struct Sections<'blocks, 'source> {
    end: usize,
    remaining: &'blocks [markdown::Block<'source>],
}

pub fn check(source: &str) -> Result<(), Error> {
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
        Ok(()) => Ok(()),
        Err(issue) => {
            let diagnostic = at_line(source, issue);
            Err(diagnostic)
        }
    }
}

fn at_line(source: &str, issue: Violation) -> Error {
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

fn separator(sections: &mut Sections<'_, '_>) -> Result<(), Violation> {
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

fn checklist(sections: &mut Sections<'_, '_>, name: &str) -> Result<(), Violation> {
    section(sections, HeadingLevel::H3, name)?;
    let message = "expected a checklist with `- [ ]` or `- [x]` items";
    let block = take(sections, message, |block| {
        matches!(block.kind, markdown::Kind::List(_))
    })?;
    let markdown::Kind::List(items) = &block.kind else {
        return invalid(block.offset, message);
    };
    for item in items {
        let prefix = match item.checked {
            Some(true) => "- [x] ",
            Some(false) => "- [ ] ",
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
    Ok(())
}

fn validate(sections: &mut Sections<'_, '_>) -> Result<(), Violation> {
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

    let mut number = 1;
    while let Some(block) = sections.remaining.first() {
        if heading(block, HeadingLevel::H2, "## Blocker") {
            break;
        }
        phase(sections, number)?;
        number += 1;
    }
    section(sections, HeadingLevel::H2, "## Blocker")?;
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
    Ok(())
}

fn phase(sections: &mut Sections<'_, '_>, number: usize) -> Result<(), Violation> {
    let prefix = format!("## Phase {number}: ");
    let message = format!("expected `## Phase {number}: <title>` or `## Blocker`");
    let prefix = prefix.as_str();
    take(sections, &message, |block| {
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
    checklist(sections, "### Todolist")?;
    checklist(sections, "### Acceptance Criteria")?;
    separator(sections)
}
