#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("expected closing `---`")]
    Closing,
    #[error("duplicate field `{0}`")]
    Duplicate(&'static str),
    #[error("unknown frontmatter field; expected `iteration` or `max_iterations`")]
    Field,
    #[error("`max_iterations` must be positive")]
    Limit,
    #[error("missing `{0}`")]
    Missing(&'static str),
    #[error("expected opening `---`")]
    Opening,
    #[error("expected a whole number for `{0}`")]
    Value(&'static str),
}

pub fn body(source: &str) -> Result<usize, (usize, Error)> {
    let mut lines = source.split_inclusive('\n');
    let Some(opening) = lines.next() else {
        return Err((0, Error::Opening));
    };
    if opening.trim_end() != "---" {
        return Err((0, Error::Opening));
    }
    let mut offset = opening.len();
    let mut iteration = None;
    let mut limit = None;
    for line in lines {
        match line.trim() {
            "---" => {
                if iteration.is_none() {
                    return Err((offset, Error::Missing("iteration")));
                }
                if limit.is_none() {
                    return Err((offset, Error::Missing("max_iterations")));
                }
                return Ok(offset + line.len());
            }
            "" => (),
            field => {
                let Some((key, value)) = field.split_once(':') else {
                    return Err((offset, Error::Field));
                };
                let (key, slot) = match key.trim_end() {
                    "iteration" => ("iteration", &mut iteration),
                    "max_iterations" => ("max_iterations", &mut limit),
                    _ => return Err((offset, Error::Field)),
                };
                if slot.is_some() {
                    return Err((offset, Error::Duplicate(key)));
                }
                let digits = value.trim();
                if !digits.bytes().all(|byte| byte.is_ascii_digit()) {
                    return Err((offset, Error::Value(key)));
                }
                let Ok(number) = digits.parse::<u64>() else {
                    return Err((offset, Error::Value(key)));
                };
                if key == "max_iterations" && number == 0 {
                    return Err((offset, Error::Limit));
                }
                *slot = Some(number);
            }
        }
        offset += line.len();
    }
    Err((offset, Error::Closing))
}
