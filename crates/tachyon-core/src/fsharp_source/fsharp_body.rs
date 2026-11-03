pub(crate) struct FsharpBody;

impl FsharpBody {
    const SWITCH_WORDS: [&'static str; 2] = ["match", "function"];
    const VARIABLE_WORDS: [&'static str; 2] = ["let", "use"];
    const DROPPED_WORDS: [&'static str; 5] = ["mutable", "fun", "then", "do", "done"];
    const OPERATORS: [(&'static str, &'static str); 4] =
        [("<>", "!="), ("<-", "="), ("->", "=>"), (":=", "=")];
    const YIELD_ALL: &'static str = "yield!";

    pub(crate) fn csharp(line: &str, code: &[bool]) -> String {
        Self::translated(line, code)
    }

    fn translated(line: &str, code: &[bool]) -> String {
        let mut output = String::with_capacity(line.len());
        let mut at = 0;
        while at < line.len() {
            let (piece, next) = Self::piece_at(line, code, at);
            output.push_str(&piece);
            at = next;
        }
        output
    }

    fn piece_at(line: &str, code: &[bool], at: usize) -> (String, usize) {
        let rest = &line[at..];
        let character = rest.chars().next().unwrap_or(' ');
        if !code.get(at).copied().unwrap_or(true) {
            (String::from(character), at + character.len_utf8())
        } else if character == '\'' {
            (String::from("_"), at + 1)
        } else if rest.starts_with(Self::YIELD_ALL) {
            (String::from("yield return"), at + Self::YIELD_ALL.len())
        } else if character.is_alphabetic() || character == '_' {
            Self::word(rest, at)
        } else if let Some((operator, replacement)) = Self::operator(rest) {
            (String::from(replacement), at + operator.len())
        } else {
            (String::from(character), at + character.len_utf8())
        }
    }

    fn word(rest: &str, at: usize) -> (String, usize) {
        let length = rest
            .find(|character: char| !Self::is_name_character(character))
            .unwrap_or(rest.len());
        let name = &rest[..length];
        (Self::replacement(name).replace('\'', "_"), at + length)
    }

    fn replacement(name: &str) -> &str {
        if Self::SWITCH_WORDS.contains(&name) {
            "switch"
        } else if Self::VARIABLE_WORDS.contains(&name) {
            "var"
        } else if Self::DROPPED_WORDS.contains(&name) {
            ""
        } else if name == "elif" {
            "else if"
        } else if name == "not" {
            "!"
        } else if name == "yield" {
            "yield return"
        } else {
            name
        }
    }

    fn is_name_character(character: char) -> bool {
        character.is_alphanumeric() || character == '_' || character == '\''
    }

    fn operator(rest: &str) -> Option<(&'static str, &'static str)> {
        Self::OPERATORS
            .iter()
            .find(|(operator, _)| rest.starts_with(operator))
            .copied()
    }
}

#[cfg(test)]
mod tests {
    use super::FsharpBody;

    fn translated(line: &str) -> String {
        FsharpBody::csharp(line, &vec![true; line.len()])
    }

    #[test]
    fn branches_and_loops_use_csharp_words() {
        assert_eq!(translated("match x with"), "switch x with");
        assert_eq!(
            translated("if a <> b then c elif d then e"),
            "if a != b  c else if d  e"
        );
        assert_eq!(translated("for item in items do"), "for item in items ");
        assert_eq!(
            translated("items |> List.map (fun x -> x + 1)"),
            "items |> List.map ( x => x + 1)"
        );
    }

    #[test]
    fn strings_and_primes_are_kept_apart() {
        let line = "let x' = \"match\"";
        let mut code = vec![true; line.len()];
        code[9..16].fill(false);
        assert_eq!(FsharpBody::csharp(line, &code), "var x_ = \"match\"");
        assert_eq!(translated("yield! more"), "yield return more");
    }
}
