use std::iter::Peekable;
use std::str::Chars;

use crate::Failure;
use crate::storage::failure;

pub(in crate::storage) struct SqlStatement;

impl SqlStatement {
    const PLACEHOLDER: char = '$';
    const FYLO_PARAMETER: char = '?';
    const NUMBERED: &'static str = "FYLO SQL parameters are $1, $2, ...";

    pub(in crate::storage) fn bound(sql: &str, count: usize) -> Result<String, Failure> {
        let mut bound = String::with_capacity(sql.len());
        let mut characters = sql.chars().peekable();
        while let Some(character) = characters.next() {
            Self::copy_one(character, &mut characters, &mut bound, count)?;
        }
        Ok(bound)
    }

    fn copy_one(
        character: char,
        characters: &mut Peekable<Chars<'_>>,
        bound: &mut String,
        count: usize,
    ) -> Result<(), Failure> {
        match character {
            '\'' | '"' => Self::copy_quoted(character, characters, bound),
            Self::PLACEHOLDER => Self::copy_placeholder(characters, bound, count),
            Self::FYLO_PARAMETER => Err(failure(Self::NUMBERED)),
            _ => {
                bound.push(character);
                Ok(())
            }
        }
    }

    fn copy_placeholder(
        characters: &mut Peekable<Chars<'_>>,
        bound: &mut String,
        count: usize,
    ) -> Result<(), Failure> {
        let index = Self::placeholder_index(characters, count)?;
        bound.push(Self::FYLO_PARAMETER);
        bound.push_str(&index.to_string());
        Ok(())
    }

    fn copy_quoted(
        quote: char,
        characters: &mut Peekable<Chars<'_>>,
        bound: &mut String,
    ) -> Result<(), Failure> {
        bound.push(quote);
        while let Some(character) = characters.next() {
            bound.push(character);
            Self::refuse_quoted_placeholder(character, characters)?;
            if character != quote {
                continue;
            }
            if characters.next_if_eq(&quote).is_none() {
                return Ok(());
            }
            bound.push(quote);
        }
        Err(failure("FYLO SQL has an unterminated quoted string."))
    }

    fn refuse_quoted_placeholder(
        character: char,
        characters: &mut Peekable<Chars<'_>>,
    ) -> Result<(), Failure> {
        let placeholder = character == Self::PLACEHOLDER;
        let numbered = characters.peek().is_some_and(char::is_ascii_digit);
        if [placeholder, numbered] == [true, true] {
            return Err(failure(
                "FYLO SQL placeholders stand alone outside quotes; write $1, not '$1'.",
            ));
        }
        Ok(())
    }

    fn placeholder_index(
        characters: &mut Peekable<Chars<'_>>,
        count: usize,
    ) -> Result<usize, Failure> {
        let mut digits = String::new();
        while let Some(digit) = characters.next_if(char::is_ascii_digit) {
            digits.push(digit);
        }
        if digits.is_empty() {
            return Err(failure(Self::NUMBERED));
        }
        digits
            .parse::<usize>()
            .ok()
            .filter(|index| (1..=count).contains(index))
            .ok_or_else(|| failure("SQL parameter index is out of range."))
    }
}
