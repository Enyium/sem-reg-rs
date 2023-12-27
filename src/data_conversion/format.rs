use std::fmt;

pub fn write_table<'a>(
    formatter: &mut fmt::Formatter<'_>,
    lines: &[Option<(&'a str, String)>],
) -> fmt::Result {
    const MIN_DOTS: usize = 2;
    let left_col_width = lines
        .iter()
        .map(|line| {
            if let Some((name, _)) = line {
                name.len()
            } else {
                0
            }
        })
        .max()
        .unwrap_or(0)
        + MIN_DOTS;

    let mut iter = lines.iter().peekable();
    while let Some(line) = iter.next() {
        let newline_suffix = if iter.peek().is_some() { "\n" } else { "" };
        if let Some((name, value)) = line {
            let dot_padding = ".".repeat(left_col_width - name.len());
            write!(formatter, "{name} {dot_padding} {value}{newline_suffix}")?;
        } else {
            write!(formatter, "{newline_suffix}")?;
        }
    }

    Ok(())
}
