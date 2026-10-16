pub(crate) struct ShippedScript;

impl ShippedScript {
    const LINE_BREAK: &'static str = "\n";
    const COMMENT_START: &'static str = "\n/*";
    const COMMENT_END: &'static str = "*/";
    const LINE_COMMENT: &'static str = "//";
    const KEPT_COMMENT_MARKS: [&'static str; 2] = ["/*!", "@license"];
    const TEMPLATE_QUOTE: char = '`';
    const LINE_CONTINUATION: char = '\\';

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

    pub(crate) fn compact_keeping_notice(source: &str) -> String {
        let mut notice = String::new();
        let mut rest = source;
        while rest.starts_with(Self::LINE_COMMENT) {
            let (line, after) = rest.split_once(Self::LINE_BREAK).unwrap_or((rest, ""));
            notice.push_str(line);
            notice.push_str(Self::LINE_BREAK);
            rest = after;
        }
        notice + &Self::compact(rest)
    }

    pub(crate) fn compacts_safely(source: &str) -> bool {
        let keeps_a_comment = Self::KEPT_COMMENT_MARKS
            .iter()
            .any(|mark| source.contains(mark));
        if keeps_a_comment {
            return false;
        }
        let continued_line = source.lines().any(Self::continues_on_next_line);
        !continued_line
    }

    fn continues_on_next_line(line: &str) -> bool {
        let unescaped = line.replace("\\\\", "").replace("\\`", "");
        let template_quotes = unescaped.matches(Self::TEMPLATE_QUOTE).count();
        let opens_or_closes_template = !template_quotes.is_multiple_of(2);
        opens_or_closes_template || line.ends_with(Self::LINE_CONTINUATION)
    }

    fn trimmed_lines(source: &str) -> String {
        let lines: Vec<&str> = source.lines().map(str::trim).collect();
        lines.join(Self::LINE_BREAK)
    }

    fn without_comment_blocks(text: &str) -> String {
        let mut pieces = text.split(Self::COMMENT_START);
        let mut kept = pieces.next().unwrap_or_default().to_owned();
        for piece in pieces {
            let Some((_, rest)) = piece.split_once(Self::COMMENT_END) else {
                continue;
            };
            kept.push_str(Self::LINE_BREAK);
            kept.push_str(rest);
        }
        kept
    }

    fn is_code(line: &str) -> bool {
        [line.is_empty(), line.starts_with(Self::LINE_COMMENT)] == [false, false]
    }
}
