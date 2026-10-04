pub(crate) struct ShippedScript;

impl ShippedScript {
    const LINE_BREAK: &'static str = "\n";
    const COMMENT_START: &'static str = "\n/*";
    const COMMENT_END: &'static str = "*/";
    const LINE_COMMENT: &'static str = "//";

    pub(crate) fn compact(source: &str) -> String {
        let trimmed = format!("{}{}", Self::LINE_BREAK, Self::trimmed_lines(source));
        let uncommented = Self::without_comment_blocks(&trimmed);
        let code: Vec<&str> = uncommented
            .lines()
            .map(str::trim)
            .filter(|line| Self::is_code(line))
            .collect();
        code.join(Self::LINE_BREAK) + Self::LINE_BREAK
    }

    fn trimmed_lines(source: &str) -> String {
        let lines: Vec<&str> = source.lines().map(str::trim).collect();
        lines.join(Self::LINE_BREAK)
    }

    fn without_comment_blocks(text: &str) -> String {
        let mut pieces = text.split(Self::COMMENT_START);
        let first = pieces.next().unwrap_or_default().to_owned();
        pieces
            .map(Self::after_comment)
            .fold(first, |mut kept, rest| {
                kept.push_str(Self::LINE_BREAK);
                kept.push_str(rest);
                kept
            })
    }

    fn after_comment(piece: &str) -> &str {
        piece
            .split_once(Self::COMMENT_END)
            .map_or(piece, |(_, rest)| rest)
    }

    fn is_code(line: &str) -> bool {
        [line.is_empty(), line.starts_with(Self::LINE_COMMENT)] == [false, false]
    }
}
