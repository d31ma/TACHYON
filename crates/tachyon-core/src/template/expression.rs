use crate::Failure;
use crate::failure::{diagnostic, source_span};
use serde_json::{Number, Value};
use std::collections::BTreeMap;
use std::str::FromStr;

pub(crate) type Scope = BTreeMap<String, Value>;

type PrefixNode = fn(Box<ExpressionNode>) -> ExpressionNode;

type OperandParser<'a> = fn(&mut Parser<'a>, usize) -> Result<ExpressionNode, Failure>;

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
        Self::check_length(source, source_path, source_start)?;
        let tokens = Lexer::new(source, source_path, source_start).tokenize()?;
        let mut parser = Parser {
            tokens: &tokens,
            current: 0,
            source_path,
            source_start,
            source_len: source.len(),
        };
        let node = parser.parse_ternary(0)?;
        let finished = parser.peek().kind == TokenKind::End;
        parser.require(
            finished,
            "Template expression contains unexpected trailing syntax.",
        )?;
        Ok(Self {
            source: String::from(source),
            node,
        })
    }

    fn check_length(source: &str, source_path: &str, source_start: usize) -> Result<(), Failure> {
        let fits = (1..=Self::MAX_EXPRESSION_BYTES).contains(&source.len());
        fits.then_some(()).ok_or_else(|| {
            Self::failure(
                source_path,
                source_start,
                source_start.saturating_add(source.len()),
                "Template expression must contain 1 through 1,024 bytes.",
            )
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
        self.node.client_json()
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

    fn failure(source_path: &str, start: usize, end: usize, message: &str) -> Failure {
        Failure::one(diagnostic(
            1303,
            message,
            Some(String::from(
                "Use bounded JSON paths, literals, comparisons, and boolean operators.",
            )),
            source_span(source_path, start, end),
        ))
    }
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

impl ExpressionNode {
    fn client_json(&self) -> String {
        match self {
            Self::Literal(value) => format!(r#"{{"k":"lit","v":{value}}}"#),
            Self::Identifier(name) => format!(r#"{{"k":"id","n":"{name}"}}"#),
            Self::Access(parent, Access::Property(property)) => format!(
                r#"{{"k":"get","o":{},"p":"{property}"}}"#,
                parent.client_json()
            ),
            Self::Access(parent, Access::Index(index)) => {
                format!(r#"{{"k":"idx","o":{},"i":{index}}}"#, parent.client_json())
            }
            Self::Not(inner) => format!(r#"{{"k":"not","e":{}}}"#, inner.client_json()),
            Self::Compare {
                left,
                operator,
                right,
            } => operator.binary_json(left, right),
            Self::Logical {
                left,
                operator,
                right,
            } => operator.binary_json(left, right),
            Self::Arithmetic {
                left,
                operator,
                right,
            } => operator.binary_json(left, right),
            Self::Conditional {
                condition,
                when_true,
                when_false,
            } => format!(
                r#"{{"k":"if","c":{},"t":{},"f":{}}}"#,
                condition.client_json(),
                when_true.client_json(),
                when_false.client_json()
            ),
            Self::Await(inner) => format!(r#"{{"k":"await","e":{}}}"#, inner.client_json()),
            Self::Call { callee, arguments } => format!(
                r#"{{"k":"call","c":{},"a":[{}]}}"#,
                callee.client_json(),
                Self::client_json_list(arguments)
            ),
        }
    }

    fn binary_client_json(kind: &str, operator: &str, left: &Self, right: &Self) -> String {
        format!(
            r#"{{"k":"{kind}","op":"{operator}","l":{},"r":{}}}"#,
            left.client_json(),
            right.client_json()
        )
    }

    fn client_json_list(nodes: &[Self]) -> String {
        let items: Vec<String> = nodes.iter().map(Self::client_json).collect();
        items.join(",")
    }

    fn compare(left: Self, operator: CompareOperator, right: Self) -> Self {
        Self::Compare {
            left: Box::new(left),
            operator,
            right: Box::new(right),
        }
    }

    fn logical(left: Self, operator: LogicalOperator, right: Self) -> Self {
        Self::Logical {
            left: Box::new(left),
            operator,
            right: Box::new(right),
        }
    }

    fn arithmetic(left: Self, operator: ArithmeticOperator, right: Self) -> Self {
        Self::Arithmetic {
            left: Box::new(left),
            operator,
            right: Box::new(right),
        }
    }
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

    fn binary_json(self, left: &ExpressionNode, right: &ExpressionNode) -> String {
        ExpressionNode::binary_client_json("num", self.client_name(), left, right)
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Access {
    Property(String),
    Index(usize),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AccessStep {
    Property,
    Call,
    Index,
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

    fn binary_json(self, left: &ExpressionNode, right: &ExpressionNode) -> String {
        ExpressionNode::binary_client_json("cmp", self.client_name(), left, right)
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

    fn binary_json(self, left: &ExpressionNode, right: &ExpressionNode) -> String {
        ExpressionNode::binary_client_json("log", self.client_name(), left, right)
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

    fn word(text: &str) -> Self {
        match text {
            "true" => Self::Literal(Value::Bool(true)),
            "false" => Self::Literal(Value::Bool(false)),
            "null" => Self::Literal(Value::Null),
            value => Self::Identifier(String::from(value)),
        }
    }

    fn prefix(&self) -> Option<PrefixNode> {
        match self {
            Self::Bang => Some(ExpressionNode::Not),
            Self::Identifier(name) if name == "await" => Some(ExpressionNode::Await),
            _ => None,
        }
    }

    fn access_step(&self) -> Option<AccessStep> {
        match self {
            Self::Dot => Some(AccessStep::Property),
            Self::LeftParen => Some(AccessStep::Call),
            Self::LeftBracket => Some(AccessStep::Index),
            _ => None,
        }
    }

    fn property_name(&self) -> Option<String> {
        let Self::Identifier(name) = self else {
            return None;
        };
        Some(name.clone())
    }

    fn number_literal(&self) -> Option<&Number> {
        let Self::Literal(Value::Number(number)) = self else {
            return None;
        };
        Some(number)
    }
}

struct Lexer<'a> {
    source: &'a str,
    source_path: &'a str,
    source_start: usize,
    current: usize,
}

impl<'a> Lexer<'a> {
    const LONG_OPERATORS: [(&'static str, TokenKind); 8] = [
        ("!==", TokenKind::NotEqual),
        ("!=", TokenKind::NotEqual),
        ("===", TokenKind::Equal),
        ("==", TokenKind::Equal),
        ("<=", TokenKind::LessEqual),
        (">=", TokenKind::GreaterEqual),
        ("&&", TokenKind::And),
        ("||", TokenKind::Or),
    ];
    const ESCAPES: [(u8, char); 6] = [
        (b'\\', '\\'),
        (b'\'', '\''),
        (b'"', '"'),
        (b'n', '\n'),
        (b'r', '\r'),
        (b't', '\t'),
    ];

    const fn new(source: &'a str, source_path: &'a str, source_start: usize) -> Self {
        Self {
            source,
            source_path,
            source_start,
            current: 0,
        }
    }

    fn identifier_start(byte: u8) -> bool {
        [byte.is_ascii_alphabetic(), byte == b'_', byte == b'$'].contains(&true)
    }

    fn identifier_continue(byte: u8) -> bool {
        [Self::identifier_start(byte), byte.is_ascii_digit()].contains(&true)
    }

    fn tokenize(mut self) -> Result<Vec<Token>, Failure> {
        let mut tokens = Vec::new();
        self.skip_whitespace();
        while self.current < self.source.len() {
            let start = self.current;
            let kind = self.token_kind()?;
            tokens.push(Token {
                kind,
                start,
                end: self.current,
            });
            self.skip_whitespace();
        }
        tokens.push(Token {
            kind: TokenKind::End,
            start: self.current,
            end: self.current,
        });
        Ok(tokens)
    }

    fn token_kind(&mut self) -> Result<TokenKind, Failure> {
        let byte = self.source.as_bytes()[self.current];
        let operator = self.operator(byte);
        operator.map_or_else(|| self.value_token(byte), Ok)
    }

    fn operator(&mut self, byte: u8) -> Option<TokenKind> {
        let long = self.long_operator();
        long.or_else(|| self.single_operator(byte))
    }

    fn long_operator(&mut self) -> Option<TokenKind> {
        let operator = Self::LONG_OPERATORS
            .into_iter()
            .find(|(text, _)| self.consume(text.as_bytes()));
        operator.map(|(_, kind)| kind)
    }

    fn single_operator(&mut self, byte: u8) -> Option<TokenKind> {
        let kind = TokenKind::single_byte(byte);
        self.current += usize::from(kind.is_some());
        kind
    }

    fn value_token(&mut self, byte: u8) -> Result<TokenKind, Failure> {
        match byte {
            b'\'' | b'"' => self.string(byte),
            b'0'..=b'9' => self.number(),
            _ if Self::identifier_start(byte) => Ok(self.identifier()),
            _ => {
                let start = self.current;
                Err(self.error(
                    start,
                    start.saturating_add(1),
                    "Template expression contains unsupported syntax.",
                ))
            }
        }
    }

    fn consume(&mut self, expected: &[u8]) -> bool {
        let end = self.current.saturating_add(expected.len());
        let found = self.source.as_bytes().get(self.current..end) == Some(expected);
        if found {
            self.current = end;
        }
        found
    }

    fn consume_one_of(&mut self, choices: &[u8]) -> bool {
        let next = self.source.as_bytes().get(self.current);
        let found = next.is_some_and(|byte| choices.contains(byte));
        self.current += usize::from(found);
        found
    }

    fn next_byte_matches(&self, test: fn(u8) -> bool) -> bool {
        let next = self.source.as_bytes().get(self.current);
        next.is_some_and(|byte| test(*byte))
    }

    fn skip_whitespace(&mut self) {
        while self.next_byte_matches(|byte| byte.is_ascii_whitespace()) {
            self.current += 1;
        }
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
        while self.inside_string(quote) {
            let character = self.string_character(start)?;
            value.push(character);
        }
        let closed = self.consume_one_of(&[quote]);
        let literal = TokenKind::Literal(Value::String(value));
        closed
            .then_some(literal)
            .ok_or_else(|| self.unterminated_string(start))
    }

    fn inside_string(&self, quote: u8) -> bool {
        let next = self.source.as_bytes().get(self.current);
        next.is_some_and(|byte| *byte != quote)
    }

    fn string_character(&mut self, string_start: usize) -> Result<char, Failure> {
        let source = self.source;
        let rest = &source[self.current..];
        if rest.starts_with('\\') {
            return self.escaped_character(string_start);
        }
        let character = rest.chars().next().unwrap_or_default();
        self.current += character.len_utf8();
        Ok(character)
    }

    fn escaped_character(&mut self, string_start: usize) -> Result<char, Failure> {
        self.current += 1;
        let escaped = self.source.as_bytes().get(self.current).copied();
        let escaped = escaped.ok_or_else(|| self.unterminated_string(string_start))?;
        self.current += 1;
        let escape = Self::ESCAPES
            .iter()
            .find(|(written, _)| *written == escaped);
        let character = escape.map(|(_, character)| *character);
        character.ok_or_else(|| {
            self.error(
                self.current.saturating_sub(2),
                self.current,
                "Template string contains an unsupported escape.",
            )
        })
    }

    fn unterminated_string(&self, string_start: usize) -> Failure {
        self.error(
            string_start,
            self.source.len(),
            "Template string is not terminated.",
        )
    }

    fn number(&mut self) -> Result<TokenKind, Failure> {
        let start = self.current;
        self.skip_digits();
        self.skip_fraction();
        self.skip_exponent();
        let raw = &self.source[start..self.current];
        let number = Number::from_str(raw).map_err(|_| {
            self.error(
                start,
                self.current,
                "Template expression contains an invalid finite number.",
            )
        })?;
        Ok(TokenKind::Literal(Value::Number(number)))
    }

    fn skip_fraction(&mut self) {
        if self.consume_one_of(b".") {
            self.skip_digits();
        }
    }

    fn skip_exponent(&mut self) {
        if self.consume_one_of(b"eE") {
            self.consume_one_of(b"+-");
            self.skip_digits();
        }
    }

    fn identifier(&mut self) -> TokenKind {
        let start = self.current;
        self.current += 1;
        while self.next_byte_matches(Self::identifier_continue) {
            self.current += 1;
        }
        TokenKind::word(&self.source[start..self.current])
    }

    fn error(&self, start: usize, end: usize, message: &str) -> Failure {
        Expression::failure(
            self.source_path,
            self.source_start.saturating_add(start),
            self.source_start.saturating_add(end),
            message,
        )
    }
}

struct Parser<'a> {
    tokens: &'a [Token],
    current: usize,
    source_path: &'a str,
    source_start: usize,
    source_len: usize,
}

impl<'a> Parser<'a> {
    const MAX_EXPRESSION_DEPTH: usize = 32;
    const MAX_CALL_ARGUMENTS: usize = 8;
    const FORBIDDEN_PROPERTIES: &'static [&'static str] =
        &["__proto__", "constructor", "prototype"];
    const OR: [(TokenKind, LogicalOperator); 1] = [(TokenKind::Or, LogicalOperator::Or)];
    const AND: [(TokenKind, LogicalOperator); 1] = [(TokenKind::And, LogicalOperator::And)];
    const COMPARISONS: [(TokenKind, CompareOperator); 6] = [
        (TokenKind::Equal, CompareOperator::Equal),
        (TokenKind::NotEqual, CompareOperator::NotEqual),
        (TokenKind::Less, CompareOperator::Less),
        (TokenKind::LessEqual, CompareOperator::LessEqual),
        (TokenKind::Greater, CompareOperator::Greater),
        (TokenKind::GreaterEqual, CompareOperator::GreaterEqual),
    ];
    const ADDITIVE: [(TokenKind, ArithmeticOperator); 2] = [
        (TokenKind::Plus, ArithmeticOperator::Add),
        (TokenKind::Minus, ArithmeticOperator::Subtract),
    ];
    const MULTIPLICATIVE: [(TokenKind, ArithmeticOperator); 2] = [
        (TokenKind::Star, ArithmeticOperator::Multiply),
        (TokenKind::Slash, ArithmeticOperator::Divide),
    ];

    fn parse_ternary(&mut self, depth: usize) -> Result<ExpressionNode, Failure> {
        self.assert_depth(depth)?;
        let condition = self.parse_or(depth + 1)?;
        if !self.eat(&TokenKind::Question) {
            return Ok(condition);
        }
        let when_true = self.parse_ternary(depth + 1)?;
        self.expect(
            &TokenKind::Colon,
            "Conditional expression requires ':' before its alternative.",
        )?;
        let when_false = self.parse_ternary(depth + 1)?;
        Ok(ExpressionNode::Conditional {
            condition: Box::new(condition),
            when_true: Box::new(when_true),
            when_false: Box::new(when_false),
        })
    }

    fn parse_or(&mut self, depth: usize) -> Result<ExpressionNode, Failure> {
        self.parse_chain(depth, &Self::OR, Self::parse_and, ExpressionNode::logical)
    }

    fn parse_and(&mut self, depth: usize) -> Result<ExpressionNode, Failure> {
        self.parse_chain(
            depth,
            &Self::AND,
            Self::parse_comparison,
            ExpressionNode::logical,
        )
    }

    fn parse_comparison(&mut self, depth: usize) -> Result<ExpressionNode, Failure> {
        self.assert_depth(depth)?;
        let left = self.parse_additive(depth + 1)?;
        let Some(operator) = self.eat_operator(&Self::COMPARISONS) else {
            return Ok(left);
        };
        let right = self.parse_additive(depth + 1)?;
        Ok(ExpressionNode::compare(left, operator, right))
    }

    fn parse_additive(&mut self, depth: usize) -> Result<ExpressionNode, Failure> {
        self.parse_chain(
            depth,
            &Self::ADDITIVE,
            Self::parse_multiplicative,
            ExpressionNode::arithmetic,
        )
    }

    fn parse_multiplicative(&mut self, depth: usize) -> Result<ExpressionNode, Failure> {
        self.parse_chain(
            depth,
            &Self::MULTIPLICATIVE,
            Self::parse_unary,
            ExpressionNode::arithmetic,
        )
    }

    fn parse_chain<T: Copy>(
        &mut self,
        depth: usize,
        operators: &[(TokenKind, T)],
        parse_operand: OperandParser<'a>,
        combine: fn(ExpressionNode, T, ExpressionNode) -> ExpressionNode,
    ) -> Result<ExpressionNode, Failure> {
        self.assert_depth(depth)?;
        let mut left = parse_operand(self, depth + 1)?;
        while let Some(operator) = self.eat_operator(operators) {
            let right = parse_operand(self, depth + 1)?;
            left = combine(left, operator, right);
        }
        Ok(left)
    }

    fn parse_unary(&mut self, depth: usize) -> Result<ExpressionNode, Failure> {
        self.assert_depth(depth)?;
        let Some(wrap) = self.eat_prefix() else {
            return self.parse_access(depth + 1);
        };
        let operand = self.parse_unary(depth + 1)?;
        Ok(wrap(Box::new(operand)))
    }

    fn parse_access(&mut self, depth: usize) -> Result<ExpressionNode, Failure> {
        self.assert_depth(depth)?;
        let mut expression = self.parse_primary(depth + 1)?;
        while let Some(step) = self.peek().kind.access_step() {
            expression = self.parse_access_step(step, expression, depth)?;
        }
        Ok(expression)
    }

    fn parse_access_step(
        &mut self,
        step: AccessStep,
        object: ExpressionNode,
        depth: usize,
    ) -> Result<ExpressionNode, Failure> {
        match step {
            AccessStep::Property => self.parse_property(object),
            AccessStep::Call => self.parse_call(object, depth),
            AccessStep::Index => self.parse_index(object),
        }
    }

    fn parse_property(&mut self, object: ExpressionNode) -> Result<ExpressionNode, Failure> {
        self.advance();
        let property = self.peek().kind.property_name();
        let property =
            property.ok_or_else(|| self.error("Property access requires an identifier."))?;
        let allowed = !Self::FORBIDDEN_PROPERTIES.contains(&property.as_str());
        self.require(
            allowed,
            "Template expressions cannot access prototype properties.",
        )?;
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
        let arguments = self.parse_arguments(depth)?;
        self.expect(&TokenKind::RightParen, "A call is missing ')'.")?;
        Ok(ExpressionNode::Call {
            callee: Box::new(callee),
            arguments,
        })
    }

    fn parse_arguments(&mut self, depth: usize) -> Result<Vec<ExpressionNode>, Failure> {
        let mut arguments = Vec::new();
        let mut expecting = self.peek().kind != TokenKind::RightParen;
        while expecting {
            let room = arguments.len() < Self::MAX_CALL_ARGUMENTS;
            self.require(room, "A call accepts at most eight arguments.")?;
            arguments.push(self.parse_ternary(depth + 1)?);
            let separated = self.eat(&TokenKind::Comma);
            let closing = self.peek().kind == TokenKind::RightParen;
            expecting = [separated, !closing] == [true, true];
        }
        Ok(arguments)
    }

    fn parse_index(&mut self, array: ExpressionNode) -> Result<ExpressionNode, Failure> {
        self.advance();
        let number =
            self.peek().kind.number_literal().ok_or_else(|| {
                self.error("Array access requires a non-negative integer literal.")
            })?;
        let index = number
            .as_u64()
            .and_then(|value| usize::try_from(value).ok());
        let index =
            index.ok_or_else(|| self.error("Array index is outside the supported range."))?;
        self.advance();
        self.expect(&TokenKind::RightBracket, "Array access is missing ']'.")?;
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
            TokenKind::LeftParen => self.parse_group(depth),
            _ => Err(self.error("Template expression is incomplete.")),
        }
    }

    fn parse_group(&mut self, depth: usize) -> Result<ExpressionNode, Failure> {
        self.advance();
        let expression = self.parse_ternary(depth + 1)?;
        self.expect(
            &TokenKind::RightParen,
            "Template expression is missing ')'.",
        )?;
        Ok(expression)
    }

    fn assert_depth(&self, depth: usize) -> Result<(), Failure> {
        self.require(
            depth <= Self::MAX_EXPRESSION_DEPTH,
            "Template expression exceeds the nesting limit of 32.",
        )
    }

    fn eat_prefix(&mut self) -> Option<PrefixNode> {
        let wrap = self.peek().kind.prefix();
        wrap.inspect(|_| self.advance())
    }

    fn eat_operator<T: Copy>(&mut self, operators: &[(TokenKind, T)]) -> Option<T> {
        let kind = &self.peek().kind;
        let found = operators.iter().find(|(token, _)| token == kind);
        let operator = found.map(|(_, operator)| *operator);
        operator.inspect(|_| self.advance())
    }

    fn eat(&mut self, kind: &TokenKind) -> bool {
        let found = self.peek().kind == *kind;
        found.then(|| self.advance()).is_some()
    }

    fn expect(&mut self, kind: &TokenKind, message: &str) -> Result<(), Failure> {
        let found = self.eat(kind);
        self.require(found, message)
    }

    fn require(&self, condition: bool, message: &str) -> Result<(), Failure> {
        condition.then_some(()).ok_or_else(|| self.error(message))
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn advance(&mut self) {
        let last = self.tokens.len() - 1;
        self.current = (self.current + 1).min(last);
    }

    fn error(&self, message: &str) -> Failure {
        let token = self.peek();
        let end = token.end.max(token.start + 1).min(self.source_len);
        Expression::failure(
            self.source_path,
            self.source_start.saturating_add(token.start),
            self.source_start.saturating_add(end),
            message,
        )
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::{Expression, Lexer};

    #[test]
    fn identifier_bytes_keep_ascii_word_and_dollar_classification() {
        for byte in u8::MIN..=u8::MAX {
            let start = byte.is_ascii_alphabetic() || byte == b'_' || byte == b'$';
            assert_eq!(Lexer::identifier_start(byte), start, "start byte {byte}");
            assert_eq!(
                Lexer::identifier_continue(byte),
                start || byte.is_ascii_digit(),
                "continue byte {byte}"
            );
        }
        for source in ["$item._value2", "_private.$0", "Item9"] {
            assert!(
                Expression::parse(source, "client/pages/tac.html", 0).is_ok(),
                "{source}"
            );
        }
    }

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
