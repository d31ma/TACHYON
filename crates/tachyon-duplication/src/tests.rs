use crate::{CodeToken, DuplicateFinder, SourceFile, TokenKind};

fn tokens(text: &str) -> Vec<CodeToken> {
    text.lines()
        .enumerate()
        .flat_map(|(index, line)| {
            line.split_whitespace()
                .map(move |word| CodeToken::new(word, index + 1, kind(word)))
        })
        .collect()
}

fn kind(word: &str) -> TokenKind {
    let first = word.chars().next().unwrap_or(' ');
    if first.is_ascii_digit() || first == '"' {
        return TokenKind::Literal;
    }
    if first.is_ascii_alphabetic() {
        return TokenKind::Name;
    }
    TokenKind::Symbol
}

fn block(name: &str, value: &str, lines: usize) -> String {
    (0..lines)
        .map(|line| format!("let {name}{line} = {name} . read ( {value} , {line} ) ;"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn file(path: &str, language: &str, text: &str) -> SourceFile {
    SourceFile::new(path, language, tokens(text))
}

#[test]
fn a_block_repeated_with_other_names_and_values_is_a_duplicate() {
    let first = format!("start ;\n{}", block("order", "\"a\"", 12));
    let second = format!("{}\nfinish ;", block("invoice", "7", 12));
    let found = DuplicateFinder::find(&[
        file("server/services/A.js", "server/js", &first),
        file("server/services/B.js", "server/js", &second),
    ]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].first.label(), "server/services/A.js:2-13");
    assert_eq!(found[0].second.label(), "server/services/B.js:1-12");
    assert_eq!(found[0].tokens, 144);
}

#[test]
fn other_languages_do_not_share_duplicates() {
    let text = block("order", "1", 12);
    let found = DuplicateFinder::find(&[
        file("server/A.js", "server/js", &text),
        file("server/A.py", "server/py", &text),
    ]);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_block_on_fewer_than_ten_lines_is_not_a_duplicate() {
    let text = block("order", "1", 12).replace('\n', " ");
    let short = format!("{text}\n{text}");
    let found = DuplicateFinder::find(&[file("server/A.js", "server/js", &short)]);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_block_with_fewer_than_one_hundred_tokens_is_not_a_duplicate() {
    let text = block("order", "1", 9);
    let found = DuplicateFinder::find(&[
        file("server/A.js", "server/js", &text),
        file("server/B.js", "server/js", &text),
    ]);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_block_repeated_in_one_file_is_a_duplicate() {
    let text = format!("{}\nmiddle ;\n{}", block("a", "1", 11), block("b", "2", 11));
    let found = DuplicateFinder::find(&[file("client/A.ts", "client/ts", &text)]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].first.first_line, 1);
    assert_eq!(found[0].second.label(), "client/A.ts:12-22");
}

#[test]
fn keywords_keep_their_spelling() {
    let first = block("order", "1", 12);
    let second = first.replace("let", "var");
    let found = DuplicateFinder::find(&[
        file("server/A.js", "server/js", &first),
        file("server/B.js", "server/js", &second),
    ]);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn every_later_copy_is_reported_once() {
    let text = block("order", "1", 12);
    let found = DuplicateFinder::find(&[
        file("server/A.js", "server/js", &text),
        file("server/B.js", "server/js", &text),
        file("server/C.js", "server/js", &text),
    ]);
    let pairs: Vec<(String, String)> = found
        .iter()
        .map(|duplicate| (duplicate.first.path.clone(), duplicate.second.path.clone()))
        .collect();
    assert_eq!(
        pairs,
        vec![
            ("server/A.js".to_owned(), "server/B.js".to_owned()),
            ("server/A.js".to_owned(), "server/C.js".to_owned()),
        ]
    );
}

#[test]
fn a_list_of_values_is_data_and_not_code() {
    let values = (0..60)
        .map(|index| format!("\"value{index}\" ,\n"))
        .collect::<Vec<_>>()
        .concat();
    let text = format!("const WORDS = [\n{values}] ;\nconst NAMES = [\n{values}] ;");
    let found = DuplicateFinder::find(&[file("server/A.js", "server/js", &text)]);
    assert!(found.is_empty(), "{found:?}");
}
