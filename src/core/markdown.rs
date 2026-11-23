use pulldown_cmark as md;

#[derive(Debug, thiserror::Error)]
#[error("invalid Markdown source range at byte {0}")]
pub struct Error(pub usize);

pub struct Block<'source> {
    pub kind: Kind<'source>,
    pub offset: usize,
    pub text: &'source str,
}

pub enum Kind<'source> {
    Heading(md::HeadingLevel),
    List(Vec<Item<'source>>),
    Other,
    Paragraph,
    Rule,
}

pub struct Item<'source> {
    pub checked: Option<bool>,
    pub has_link: bool,
    pub offset: usize,
    pub text: &'source str,
}

pub fn blocks(source: &str, offset: usize) -> Result<Vec<Block<'_>>, Error> {
    let body_error = Error(offset);
    let body = source.get(offset..).ok_or(body_error)?;
    let events = md::Parser::new_ext(body, md::Options::ENABLE_TASKLISTS).into_offset_iter();
    let mut depth: usize = 0;
    let mut blocks = Vec::new();
    for (event, span) in events {
        let start = offset + span.start;
        let range_error = Error(start);
        match event {
            md::Event::Start(tag) if depth == 0 => {
                let text = body.get(span).ok_or(range_error)?;
                let kind = block_kind(&tag);
                blocks.push(Block {
                    kind,
                    offset: start,
                    text,
                });
                depth += 1;
            }
            md::Event::Start(tag) => {
                if let Some(Block {
                    kind: Kind::List(items),
                    ..
                }) = blocks.last_mut()
                {
                    if matches!(tag, md::Tag::Item) {
                        let text = body.get(span).ok_or(range_error)?;
                        items.push(Item {
                            offset: start,
                            text,
                            checked: None,
                            has_link: false,
                        });
                    }
                    if matches!(tag, md::Tag::Link { .. })
                        && let Some(item) = items.last_mut()
                    {
                        item.has_link = true;
                    }
                }
                depth += 1;
            }
            md::Event::End(_) => depth -= 1,
            md::Event::TaskListMarker(checked) => {
                if let Some(Block {
                    kind: Kind::List(items),
                    ..
                }) = blocks.last_mut()
                    && let Some(item) = items.last_mut()
                {
                    item.checked = Some(checked);
                }
            }
            md::Event::Rule if depth == 0 => {
                let text = body.get(span).ok_or(range_error)?;
                blocks.push(Block {
                    offset: start,
                    text,
                    kind: Kind::Rule,
                });
            }
            md::Event::Text(_)
            | md::Event::Code(_)
            | md::Event::InlineMath(_)
            | md::Event::DisplayMath(_)
            | md::Event::Html(_)
            | md::Event::InlineHtml(_)
            | md::Event::FootnoteReference(_)
            | md::Event::SoftBreak
            | md::Event::HardBreak
            | md::Event::Rule => (),
        }
    }
    Ok(blocks)
}

fn block_kind<'source>(tag: &md::Tag<'_>) -> Kind<'source> {
    if let md::Tag::Heading { level, .. } = tag {
        return Kind::Heading(*level);
    }
    if matches!(tag, md::Tag::Paragraph) {
        return Kind::Paragraph;
    }
    if matches!(tag, md::Tag::List(None)) {
        let items = Vec::new();
        return Kind::List(items);
    }
    Kind::Other
}
