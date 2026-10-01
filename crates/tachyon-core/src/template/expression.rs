use crate::Failure;
use crate::failure::{diagnostic, source_span};
use serde_json::{Number, Value};
use std::collections::BTreeMap;
use std::str::FromStr;

pub(crate) type Scope = BTreeMap<String, Value>;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Expression {
    source: String,
    node: ExpressionNode,
}

impl Expression {
    const MAX_EXPRESSION_BYTES: usize = 1_024;

    pub(crate) fn parse(
        source: &str,
        source_path: &str,
        source_start: usize,
    ) -> Result<Self, Failure> {
        if source.is_empty() || source.len() > Self::MAX_EXPRESSION_BYTES {
            return Err(expression_failure(
                source_path,
                source_start,
                source_start.saturating_add(source.len()),
                "Template expression must contain 1 through 1,024 bytes.",
            ));
        }
        let tokens = Lexer::new(source, source_path, source_start).tokenize()?;
        let mut parser = Parser {
            tokens: &tokens,
            current: 0,
            source_path,
            source_start,
            source_len: source.len(),
        };
        let node = parser.parse_ternary(0)?;
        if !matches!(parser.peek().kind, TokenKind::End) {
            return Err(parser.error("Template expression contains unexpected trailing syntax."));
        }
        Ok(Self {
            source: String::from(source),
            node,
        })
    }

    pub(crate) fn source(&self) -> &str {
        &self.source
    }

    pub(crate) fn identifier_name(&self) -> Option<&str> {
        self.bare_identifier()
    }

    fn bare_identifier(&self) -> Option<&str> {
        match &self.node {
            ExpressionNode::Identifier(name) => Some(name),
            _ => None,
        }
    }

    pub(crate) fn to_client_json(&self) -> String {
        client_json(&self.node)
    }

    pub(crate) fn equals(&self, other: &Self) -> Self {
        Self {
            source: format!("{} == {}", self.source, other.source),
            node: ExpressionNode::Compare {
                left: Box::new(self.node.clone()),
                operator: CompareOperator::Equal,
                right: Box::new(other.node.clone()),
            },
        }
    }
}

fn client_json(node: &ExpressionNode) -> String {
    match node {
        ExpressionNode::Literal(value) => format!(r#"{{"k":"lit","v":{value}}}"#),
        ExpressionNode::Identifier(name) => format!(r#"{{"k":"id","n":"{name}"}}"#),
        ExpressionNode::Access(parent, Access::Property(property)) => format!(
            r#"{{"k":"get","o":{},"p":"{property}"}}"#,
            client_json(parent)
        ),
        ExpressionNode::Access(parent, Access::Index(index)) => {
            format!(r#"{{"k":"idx","o":{},"i":{index}}}"#, client_json(parent))
        }
        ExpressionNode::Not(inner) => format!(r#"{{"k":"not","e":{}}}"#, client_json(inner)),
        ExpressionNode::Compare {
            left,
            operator,
            right,
        } => binary_client_json("cmp", operator.client_name(), left, right),
        ExpressionNode::Logical {
            left,
            operator,
            right,
        } => binary_client_json("log", operator.client_name(), left, right),
        ExpressionNode::Arithmetic {
            left,
            operator,
            right,
        } => binary_client_json("num", operator.client_name(), left, right),
        ExpressionNode::Conditional {
            condition,
            when_true,
            when_false,
        } => format!(
            r#"{{"k":"if","c":{},"t":{},"f":{}}}"#,
            client_json(condition),
            client_json(when_true),
            client_json(when_false)
        ),
        ExpressionNode::Await(inner) => format!(r#"{{"k":"await","e":{}}}"#, client_json(inner)),
        ExpressionNode::Call { callee, arguments } => format!(
            r#"{{"k":"call","c":{},"a":[{}]}}"#,
            client_json(callee),
            client_json_list(arguments)
        ),
    }
}

fn binary_client_json(
    kind: &str,
    operator: &str,
    left: &ExpressionNode,
    right: &ExpressionNode,
) -> String {
    format!(
        r#"{{"k":"{kind}","op":"{operator}","l":{},"r":{}}}"#,
        client_json(left),
        client_json(right)
    )
}

fn client_json_list(nodes: &[ExpressionNode]) -> String {
    let items: Vec<String> = nodes.iter().map(client_json).collect();
    items.join(",")
}

#[derive(Clone, Debug, PartialEq)]
enum ExpressionNode {
    Literal(Value),
    Identifier(String),
    Access(Box<Self>, Access),
    Not(Box<Self>),
    Compare {
        left: Box<Self>,
        operator: CompareOperator,
        right: Box<Self>,
    },
    Logical {
        left: Box<Self>,
        operator: LogicalOperator,
        right: Box<Self>,
    },
    Arithmetic {
        left: Box<Self>,
        operator: ArithmeticOperator,
        right: Box<Self>,
    },
    Conditional {
        condition: Box<Self>,
        when_true: Box<Self>,
        when_false: Box<Self>,
    },
    Call {
        callee: Box<Self>,
        arguments: Vec<Self>,
    },
    Await(Box<Self>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ArithmeticOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
}

impl ArithmeticOperator {
    const fn client_name(self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Subtract => "sub",
            Self::Multiply => "mul",
            Self::Divide => "div",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Access {
    Property(String),
    Index(usize),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CompareOperator {
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}

impl CompareOperator {
    const fn client_name(self) -> &'static str {
        match self {
            Self::Equal => "eq",
            Self::NotEqual => "ne",
            Self::Less => "lt",
            Self::LessEqual => "le",
            Self::Greater => "gt",
            Self::GreaterEqual => "ge",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LogicalOperator {
    And,
    Or,
}

impl LogicalOperator {
    const fn client_name(self) -> &'static str {
        match self {
            Self::And => "and",
            Self::Or => "or",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Token {
    kind: TokenKind,
    start: usize,
    end: usize,
}

#[derive(Clone, Debug, PartialEq)]
enum TokenKind {
    Identifier(String),
    Literal(Value),
    Bang,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
    Dot,
    LeftBracket,
    RightBracket,
    LeftParen,
    RightParen,
    Question,
    Colon,
    Comma,
    Plus,
    Minus,
    Star,
    Slash,
    End,
}

impl TokenKind {
    fn single_byte(byte: u8) -> Option<Self> {
        match byte {
            b'!' => Some(Self::Bang),
            b'?' => Some(Self::Question),
            b',' => Some(Self::Comma),
            b':' => Some(Self::Colon),
            b'+' => Some(Self::Plus),
            b'-' => Some(Self::Minus),
            b'*' => Some(Self::Star),
            b'/' => Some(Self::Slash),
            b'<' => Some(Self::Less),
            b'>' => Some(Self::Greater),
            b'.' => Some(Self::Dot),
            b'[' => Some(Self::LeftBracket),
            b']' => Some(Self::RightBracket),
            b'(' => Some(Self::LeftParen),
            b')' => Some(Self::RightParen),
            _ => None,
        }
    }
}

struct Lexer<'a> {
    source: &'a str,
    source_path: &'a str,
    source_start: usize,
    current: usize,
}

impl<'a> Lexer<'a> {
    const fn new(source: &'a str, source_path: &'a str, source_start: usize) -> Self {
        Self {
            source,
            source_path,
            source_start,
            current: 0,
        }
    }

    fn tokenize(mut self) -> Result<Vec<Token>, Failure> {
        let mut tokens = Vec::new();
        while self.current < self.source.len() {
            let byte = self.source.as_bytes()[self.current];
            if byte.is_ascii_whitespace() {
                self.current += 1;
                continue;
            }
            let start = self.current;
            let kind = self.token_kind(byte)?;
            tokens.push(Token {
                kind,
                start,
                end: self.current,
            });
        }
        tokens.push(Token {
            kind: TokenKind::End,
            start: self.current,
            end: self.current,
        });
        Ok(tokens)
    }

    fn token_kind(&mut self, byte: u8) -> Result<TokenKind, Failure> {
        if let Some(kind) = self.long_operator() {
            return Ok(kind);
        }
        if let Some(kind) = TokenKind::single_byte(byte) {
            self.current += 1;
            return Ok(kind);
        }
        if byte == b'\'' || byte == b'"' {
            return self.string(byte);
        }
        if byte.is_ascii_digit() {
            return self.number();
        }
        if identifier_start(byte) {
            return Ok(self.identifier());
        }
        let start = self.current;
        Err(self.error(
            start,
            start.saturating_add(1),
            "Template expression contains unsupported syntax.",
        ))
    }

    fn long_operator(&mut self) -> Option<TokenKind> {
        let operators = [
            ("!==", TokenKind::NotEqual),
            ("!=", TokenKind::NotEqual),
            ("===", TokenKind::Equal),
            ("==", TokenKind::Equal),
            ("<=", TokenKind::LessEqual),
            (">=", TokenKind::GreaterEqual),
            ("&&", TokenKind::And),
            ("||", TokenKind::Or),
        ];
        for (text, kind) in operators {
            if self.consume(text.as_bytes()) {
                return Some(kind);
            }
        }
        None
    }

    fn consume(&mut self, expected: &[u8]) -> bool {
        let end = self.current.saturating_add(expected.len());
        if self.source.as_bytes().get(self.current..end) != Some(expected) {
            return false;
        }
        self.current = end;
        true
    }

    fn consume_one_of(&mut self, choices: &[u8]) -> bool {
        let Some(byte) = self.source.as_bytes().get(self.current) else {
            return false;
        };
        if !choices.contains(byte) {
            return false;
        }
        self.current += 1;
        true
    }

    fn next_byte_matches(&self, test: fn(u8) -> bool) -> bool {
        let next = self.source.as_bytes().get(self.current);
        next.is_some_and(|byte| test(*byte))
    }

    fn skip_digits(&mut self) {
        while self.next_byte_matches(|byte| byte.is_ascii_digit()) {
            self.current += 1;
        }
    }

    fn string(&mut self, quote: u8) -> Result<TokenKind, Failure> {
        let start = self.current;
        self.current += 1;
        let mut value = String::new();
        while self.current < self.source.len() {
            let byte = self.source.as_bytes()[self.current];
            self.current += 1;
            if byte == quote {
                return Ok(TokenKind::Literal(Value::String(value)));
            }
            if byte == b'\\' {
                let Some(escaped) = self.source.as_bytes().get(self.current).copied() else {
                    break;
                };
                self.current += 1;
                let Some(character) = escaped_character(escaped) else {
                    return Err(self.error(
                        self.current.saturating_sub(2),
                        self.current,
                        "Template string contains an unsupported escape.",
                    ));
                };
                value.push(character);
                continue;
            }
            let Some(character) = self.source[self.current - 1..].chars().next() else {
                break;
            };
            value.push(character);
            self.current = self.current.saturating_sub(1) + character.len_utf8();
        }
        Err(self.error(
            start,
            self.source.len(),
            "Template string is not terminated.",
        ))
    }

    fn number(&mut self) -> Result<TokenKind, Failure> {
        let start = self.current;
        self.skip_digits();
        if self.consume(b".") {
            self.skip_digits();
        }
        if self.consume_one_of(b"eE") {
            self.consume_one_of(b"+-");
            self.skip_digits();
        }
        let raw = &self.source[start..self.current];
        let Ok(number) = Number::from_str(raw) else {
            return Err(self.error(
                start,
                self.current,
                "Template expression contains an invalid finite number.",
            ));
        };
        Ok(TokenKind::Literal(Value::Number(number)))
    }

    fn identifier(&mut self) -> TokenKind {
        let start = self.current;
        self.current += 1;
        while self.next_byte_matches(identifier_continue) {
            self.current += 1;
        }
        match &self.source[start..self.current] {
            "true" => TokenKind::Literal(Value::Bool(true)),
            "false" => TokenKind::Literal(Value::Bool(false)),
            "null" => TokenKind::Literal(Value::Null),
            value => TokenKind::Identifier(String::from(value)),
        }
    }

    fn error(&self, start: usize, end: usize, message: &str) -> Failure {
        expression_failure(
            self.source_path,
            self.source_start.saturating_add(start),
            self.source_start.saturating_add(end),
            message,
        )
    }
}

fn escaped_character(escaped: u8) -> Option<char> {
    match escaped {
        b'\\' => Some('\\'),
        b'\'' => Some('\''),
        b'"' => Some('"'),
        b'n' => Some('\n'),
        b'r' => Some('\r'),
        b't' => Some('\t'),
        _ => None,
    }
}

struct Parser<'a> {
    tokens: &'a [Token],
    current: usize,
    source_path: &'a str,
    source_start: usize,
    source_len: usize,
}

impl Parser<'_> {
    const MAX_EXPRESSION_DEPTH: usize = 32;
    const MAX_CALL_ARGUMENTS: usize = 8;
    const FORBIDDEN_PROPERTIES: &'static [&'static str] =
        &["__proto__", "constructor", "prototype"];

    fn parse_ternary(&mut self, depth: usize) -> Result<ExpressionNode, Failure> {
        self.assert_depth(depth)?;
        let condition = self.parse_or(depth + 1)?;
        if !matches!(self.peek().kind, TokenKind::Question) {
            return Ok(condition);
        }
        self.advance();
        let when_true = self.parse_ternary(depth + 1)?;
        if !matches!(self.peek().kind, TokenKind::Colon) {
            return Err(self.error("Conditional expression requires ':' before its alternative."));
        }
        self.advance();
        let when_false = self.parse_ternary(depth + 1)?;
        Ok(ExpressionNode::Conditional {
            condition: Box::new(condition),
            when_true: Box::new(when_true),
            when_false: Box::new(when_false),
        })
    }

    fn parse_or(&mut self, depth: usize) -> Result<ExpressionNode, Failure> {
        self.assert_depth(depth)?;
        let mut expression = self.parse_and(depth + 1)?;
        while matches!(self.peek().kind, TokenKind::Or) {
            self.advance();
            let right = self.parse_and(depth + 1)?;
            expression = ExpressionNode::Logical {
                left: Box::new(expression),
                operator: LogicalOperator::Or,
                right: Box::new(right),
            };
        }
        Ok(expression)
    }

    fn parse_and(&mut self, depth: usize) -> Result<ExpressionNode, Failure> {
        self.assert_depth(depth)?;
        let mut expression = self.parse_comparison(depth + 1)?;
        while matches!(self.peek().kind, TokenKind::And) {
            self.advance();
            let right = self.parse_comparison(depth + 1)?;
            expression = ExpressionNode::Logical {
                left: Box::new(expression),
                operator: LogicalOperator::And,
                right: Box::new(right),
            };
        }
        Ok(expression)
    }

    fn parse_comparison(&mut self, depth: usize) -> Result<ExpressionNode, Failure> {
        self.assert_depth(depth)?;
        let left = self.parse_additive(depth + 1)?;
        let operator = match self.peek().kind {
            TokenKind::Equal => Some(CompareOperator::Equal),
            TokenKind::NotEqual => Some(CompareOperator::NotEqual),
            TokenKind::Less => Some(CompareOperator::Less),
            TokenKind::LessEqual => Some(CompareOperator::LessEqual),
            TokenKind::Greater => Some(CompareOperator::Greater),
            TokenKind::GreaterEqual => Some(CompareOperator::GreaterEqual),
            _ => None,
        };
        let Some(operator) = operator else {
            return Ok(left);
        };
        self.advance();
        let right = self.parse_additive(depth + 1)?;
        Ok(ExpressionNode::Compare {
            left: Box::new(left),
            operator,
            right: Box::new(right),
        })
    }

    fn parse_additive(&mut self, depth: usize) -> Result<ExpressionNode, Failure> {
        self.assert_depth(depth)?;
        let mut left = self.parse_multiplicative(depth + 1)?;
        loop {
            let operator = match self.peek().kind {
                TokenKind::Plus => ArithmeticOperator::Add,
                TokenKind::Minus => ArithmeticOperator::Subtract,
                _ => return Ok(left),
            };
            self.advance();
            let right = self.parse_multiplicative(depth + 1)?;
            left = ExpressionNode::Arithmetic {
                left: Box::new(left),
                operator,
                right: Box::new(right),
            };
        }
    }

    fn parse_multiplicative(&mut self, depth: usize) -> Result<ExpressionNode, Failure> {
        self.assert_depth(depth)?;
        let mut left = self.parse_unary(depth + 1)?;
        loop {
            let operator = match self.peek().kind {
                TokenKind::Star => ArithmeticOperator::Multiply,
                TokenKind::Slash => ArithmeticOperator::Divide,
                _ => return Ok(left),
            };
            self.advance();
            let right = self.parse_unary(depth + 1)?;
            left = ExpressionNode::Arithmetic {
                left: Box::new(left),
                operator,
                right: Box::new(right),
            };
        }
    }

    fn parse_unary(&mut self, depth: usize) -> Result<ExpressionNode, Failure> {
        self.assert_depth(depth)?;
        if matches!(&self.peek().kind, TokenKind::Identifier(name) if name == "await") {
            self.advance();
            return Ok(ExpressionNode::Await(Box::new(
                self.parse_unary(depth + 1)?,
            )));
        }
        if matches!(self.peek().kind, TokenKind::Bang) {
            self.advance();
            return Ok(ExpressionNode::Not(Box::new(self.parse_unary(depth + 1)?)));
        }
        self.parse_access(depth + 1)
    }

    fn parse_access(&mut self, depth: usize) -> Result<ExpressionNode, Failure> {
        self.assert_depth(depth)?;
        let mut expression = self.parse_primary(depth + 1)?;
        loop {
            expression = match self.peek().kind {
                TokenKind::Dot => self.parse_property(expression)?,
                TokenKind::LeftParen => self.parse_call(expression, depth)?,
                TokenKind::LeftBracket => self.parse_index(expression)?,
                _ => return Ok(expression),
            };
        }
    }

    fn parse_property(&mut self, object: ExpressionNode) -> Result<ExpressionNode, Failure> {
        self.advance();
        let TokenKind::Identifier(property) = self.peek().kind.clone() else {
            return Err(self.error("Property access requires an identifier."));
        };
        if Self::FORBIDDEN_PROPERTIES.contains(&property.as_str()) {
            return Err(self.error("Template expressions cannot access prototype properties."));
        }
        self.advance();
        Ok(ExpressionNode::Access(
            Box::new(object),
            Access::Property(property),
        ))
    }

    fn parse_call(
        &mut self,
        callee: ExpressionNode,
        depth: usize,
    ) -> Result<ExpressionNode, Failure> {
        self.advance();
        let mut arguments = Vec::new();
        while !matches!(self.peek().kind, TokenKind::RightParen) {
            if arguments.len() >= Self::MAX_CALL_ARGUMENTS {
                return Err(self.error("A call accepts at most eight arguments."));
            }
            arguments.push(self.parse_ternary(depth + 1)?);
            if !matches!(self.peek().kind, TokenKind::Comma) {
                break;
            }
            self.advance();
        }
        if !matches!(self.peek().kind, TokenKind::RightParen) {
            return Err(self.error("A call is missing ')'."));
        }
        self.advance();
        Ok(ExpressionNode::Call {
            callee: Box::new(callee),
            arguments,
        })
    }

    fn parse_index(&mut self, array: ExpressionNode) -> Result<ExpressionNode, Failure> {
        self.advance();
        let TokenKind::Literal(Value::Number(number)) = self.peek().kind.clone() else {
            return Err(self.error("Array access requires a non-negative integer literal."));
        };
        let index = number
            .as_u64()
            .and_then(|value| usize::try_from(value).ok());
        let Some(index) = index else {
            return Err(self.error("Array index is outside the supported range."));
        };
        self.advance();
        if !matches!(self.peek().kind, TokenKind::RightBracket) {
            return Err(self.error("Array access is missing ']'."));
        }
        self.advance();
        Ok(ExpressionNode::Access(
            Box::new(array),
            Access::Index(index),
        ))
    }

    fn parse_primary(&mut self, depth: usize) -> Result<ExpressionNode, Failure> {
        self.assert_depth(depth)?;
        match self.peek().kind.clone() {
            TokenKind::Literal(value) => {
                self.advance();
                Ok(ExpressionNode::Literal(value))
            }
            TokenKind::Identifier(identifier) => {
                self.advance();
                Ok(ExpressionNode::Identifier(identifier))
            }
            TokenKind::LeftParen => {
                self.advance();
                let expression = self.parse_ternary(depth + 1)?;
                if !matches!(self.peek().kind, TokenKind::RightParen) {
                    return Err(self.error("Template expression is missing ')'."));
                }
                self.advance();
                Ok(expression)
            }
            _ => Err(self.error("Template expression is incomplete.")),
        }
    }

    fn assert_depth(&self, depth: usize) -> Result<(), Failure> {
        if depth > Self::MAX_EXPRESSION_DEPTH {
            return Err(self.error("Template expression exceeds the nesting limit of 32."));
        }
        Ok(())
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn advance(&mut self) {
        if self.current + 1 < self.tokens.len() {
            self.current += 1;
        }
    }

    fn error(&self, message: &str) -> Failure {
        let token = self.peek();
        let end = token.end.max(token.start + 1).min(self.source_len);
        expression_failure(
            self.source_path,
            self.source_start.saturating_add(token.start),
            self.source_start.saturating_add(end),
            message,
        )
    }
}

fn identifier_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || matches!(byte, b'_' | b'$')
}

fn identifier_continue(byte: u8) -> bool {
    identifier_start(byte) || byte.is_ascii_digit()
}

fn expression_failure(source_path: &str, start: usize, end: usize, message: &str) -> Failure {
    Failure::one(diagnostic(
        1303,
        message,
        Some(String::from(
            "Use bounded JSON paths, literals, comparisons, and boolean operators.",
        )),
        source_span(source_path, start, end),
    ))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::Expression;

    #[test]
    fn valid_expressions_serialize_to_client_ast() {
        for source in [
            "product.name",
            "product.prices[1]",
            "active && product.prices[0] < 4",
            "slug === active ? 'on' : 'off'",
            "'/atlas/' + slug",
            "await note()",
        ] {
            let expression = Expression::parse(source, "client/pages/tac.html", 0).expect(source);
            let plan = expression.to_client_json();
            assert!(plan.starts_with('{'), "{source}: {plan}");
            assert!(!plan.contains(source), "authored source leaked: {source}");
        }
    }

    #[test]
    fn unsupported_malformed_and_excessive_expressions_fail() {
        for source in [
            "",
            "value = 1",
            "object.__proto__",
            "items[-1]",
            "items[0",
            "items[value]",
            "object.",
            "(true",
            "'bad\\q'",
            "1e",
            "'unterminated",
            &"(".repeat(34),
            &"x".repeat(1_025),
        ] {
            assert!(
                Expression::parse(source, "client/pages/tac.html", 0).is_err(),
                "{source}"
            );
        }
    }
}
