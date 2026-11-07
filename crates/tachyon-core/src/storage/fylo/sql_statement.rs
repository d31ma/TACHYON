use std::iter::Peekable;
use std::str::Chars;

use serde_json::{Number, Value};

use crate::Failure;
use crate::storage::failure;

pub(in crate::storage) struct SqlStatement;

impl SqlStatement {
    pub(in crate::storage) fn rendered(sql: &str, parameters: &[Value]) -> Result<String, Failure> {
        let mut rendered = String::with_capacity(sql.len());
        let mut characters = sql.chars().peekable();
        while let Some(character) = characters.next() {
            match character {
                '\'' | '"' => Self::copy_quoted(character, &mut characters, &mut rendered)?,
                '$' => {
                    let index = Self::placeholder_index(&mut characters, parameters.len())?;
                    rendered.push(' ');
                    rendered.push_str(&Self::literal(&parameters[index - 1])?);
                    rendered.push(' ');
                }
                _ => rendered.push(character),
            }
        }
        Ok(rendered)
    }

    fn copy_quoted(
        quote: char,
        characters: &mut Peekable<Chars<'_>>,
        rendered: &mut String,
    ) -> Result<(), Failure> {
        rendered.push(quote);
        while let Some(character) = characters.next() {
            rendered.push(character);
            if character == '$' && characters.peek().is_some_and(char::is_ascii_digit) {
                return Err(failure(
                    "FYLO SQL placeholders stand alone outside quotes; write $1, not '$1'.",
                ));
            }
            if character != quote {
                continue;
            }
            if characters.next_if_eq(&quote).is_none() {
                return Ok(());
            }
            rendered.push(quote);
        }
        Err(failure("FYLO SQL has an unterminated quoted string."))
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
            return Err(failure("FYLO SQL parameters are $1, $2, ..."));
        }
        digits
            .parse::<usize>()
            .ok()
            .filter(|index| (1..=count).contains(index))
            .ok_or_else(|| failure("SQL parameter index is out of range."))
    }

    fn literal(value: &Value) -> Result<String, Failure> {
        match value {
            Value::Null => Ok(String::from("NULL")),
            Value::Bool(value) => Ok(value.to_string()),
            Value::Number(number) => Self::number(number),
            Value::String(text) => Ok(format!("'{}'", text.replace('\'', "''"))),
            _ => Err(failure("SQL parameters are scalar values.")),
        }
    }

    fn number(number: &Number) -> Result<String, Failure> {
        let written = number.to_string();
        let plain = written
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b'.');
        plain.then_some(written).ok_or_else(|| {
            failure("FYLO SQL numbers are non-negative decimals without an exponent.")
        })
    }
}
