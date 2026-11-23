use std::num;

use super::markdown;
use super::plan::{
    Error, PHASE_PREFIX, Sections, Violation, append, at_line, check, line_start, parse, phase,
    separator,
};

#[derive(Clone, Copy)]
pub enum Input<'source> {
    Fragment(&'source str),
    Title(&'source str),
}

pub fn insert(
    source: &str,
    position: num::NonZeroUsize,
    input: Input<'_>,
) -> Result<String, Error> {
    let plan = parse(source)?;
    let position = position.get();
    let last = plan.phases.len() + 1;
    if position > last {
        return Err(Error::Position { position, last });
    }
    let offset = if position == last {
        plan.blocker
    } else {
        let following = plan.phases.get(position - 1).ok_or(Error::EditRange)?;
        following.start
    };
    let start = line_start(source, offset)?;
    let newline = if source
        .split_once('\n')
        .is_some_and(|(line, _)| line.ends_with('\r'))
    {
        "\r\n"
    } else {
        "\n"
    };
    let new_phase = match input {
        Input::Fragment(fragment) => import(fragment, position, newline)?,
        Input::Title(title) => starter(title, position, newline)?,
    };
    let mut updated = String::new();
    append(&mut updated, source, 0..start)?;
    updated.push_str(&new_phase);
    let mut cursor = start;
    for (index, following) in plan.phases.iter().enumerate().skip(position - 1) {
        append(&mut updated, source, cursor..following.number.start)?;
        let number = index + 2;
        let digits = number.to_string();
        updated.push_str(&digits);
        cursor = following.number.end;
    }
    let end = source.len();
    append(&mut updated, source, cursor..end)?;
    check(&updated)?;
    Ok(updated)
}

fn starter(title: &str, position: usize, newline: &str) -> Result<String, Error> {
    let title = title.trim();
    if title.is_empty() || title.contains(['\r', '\n']) {
        return Err(Error::Title);
    }
    let heading = format!("{PHASE_PREFIX}{position}: {title}");
    let lines = [
        heading.as_str(),
        "",
        "Describe the outcome of this phase.",
        "",
        "### Specs",
        "",
        "### Files",
        "",
        "### Todolist",
        "",
        "- [ ] Define the implementation steps.",
        "",
        "### Acceptance Criteria",
        "",
        "- [ ] Define how to verify the outcome.",
        "",
        "---",
        "",
        "",
    ];
    let text = lines.join(newline);
    Ok(text)
}

fn import(source: &str, position: usize, newline: &str) -> Result<String, Error> {
    let blocks = markdown::blocks(source, 0)?;
    let block = blocks.first().ok_or(Error::FragmentHeading)?;
    let number = block
        .text
        .strip_prefix(PHASE_PREFIX)
        .and_then(|text| text.split_once(':'))
        .map(|(digits, _)| digits)
        .ok_or(Error::FragmentHeading)?;
    let Ok(number) = number.parse::<num::NonZeroUsize>() else {
        return Err(Error::FragmentHeading);
    };
    let end = source.len();
    let mut sections = Sections {
        end,
        remaining: &blocks,
    };
    let number = number.get();
    let phase = match phase(&mut sections, number) {
        Ok(phase) => phase,
        Err(issue) => return fragment_error(source, issue),
    };
    let needs_separator = sections.remaining.is_empty();
    if !needs_separator && let Err(issue) = separator(&mut sections) {
        return fragment_error(source, issue);
    }
    if !sections.remaining.is_empty() {
        return Err(Error::FragmentCount);
    }
    let mut updated = String::new();
    append(&mut updated, source, 0..phase.number.start)?;
    let digits = position.to_string();
    updated.push_str(&digits);
    append(&mut updated, source, phase.number.end..end)?;
    if needs_separator {
        separate(&mut updated, newline);
        updated.push_str("---");
    }
    separate(&mut updated, newline);
    Ok(updated)
}

fn fragment_error<T>(source: &str, issue: Violation) -> Result<T, Error> {
    let diagnostic = at_line(source, issue);
    let error = Box::new(diagnostic);
    let fragment = Error::Fragment(error);
    Err(fragment)
}

fn separate(text: &mut String, newline: &str) {
    if !text.ends_with('\n') {
        text.push_str(newline);
    }
    let without_newline = text.trim_end_matches('\n');
    let without_cr = without_newline.strip_suffix('\r');
    if !text.ends_with("\n\n") && !without_cr.is_some_and(|line| line.ends_with('\n')) {
        text.push_str(newline);
    }
}
