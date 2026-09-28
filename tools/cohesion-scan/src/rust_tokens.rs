use crate::structure::Structure;
use proc_macro2::{Delimiter, Group, Span, TokenStream, TokenTree};
use quote::ToTokens;
use syn::{File, Item};
use tachyon_duplication::{CodeToken, SourceFile, TokenKind};

pub(crate) struct RustTokens {
    tokens: Vec<CodeToken>,
}

impl RustTokens {
    const LANGUAGE: &'static str = "rs";

    pub(crate) fn source_file(path: &str, file: &File) -> SourceFile {
        let mut reader = Self { tokens: Vec::new() };
        file.items
            .iter()
            .filter(|item| Self::is_counted(item))
            .for_each(|item| reader.add_stream(item.to_token_stream()));
        SourceFile::new(path, Self::LANGUAGE, reader.tokens)
    }

    fn is_counted(item: &Item) -> bool {
        Self::is_code_item(item) & Structure::is_counted(item)
    }

    fn is_code_item(item: &Item) -> bool {
        match item {
            Item::Use(_) | Item::ExternCrate(_) => false,
            Item::Mod(module) => module.content.is_some(),
            _ => true,
        }
    }

    fn add_stream(&mut self, stream: TokenStream) {
        let trees: Vec<TokenTree> = stream.into_iter().collect();
        let mut at = 0;
        while at < trees.len() {
            let width = Self::attribute_width(&trees, at);
            self.add_trees(&trees[at..at + usize::from(width == 0)]);
            at += width.max(1);
        }
    }

    fn add_trees(&mut self, trees: &[TokenTree]) {
        for tree in trees {
            self.add_tree(tree);
        }
    }

    fn attribute_width(trees: &[TokenTree], at: usize) -> usize {
        let hash = Self::is_punct(trees.get(at), '#');
        let bang = Self::is_punct(trees.get(at + 1), '!');
        let group_at = at + 1 + usize::from(bang);
        let bracket = Self::is_bracket_group(trees.get(group_at));
        usize::from(hash & bracket) * (group_at + 1 - at)
    }

    fn is_punct(tree: Option<&TokenTree>, wanted: char) -> bool {
        matches!(tree, Some(TokenTree::Punct(punct)) if punct.as_char() == wanted)
    }

    fn is_bracket_group(tree: Option<&TokenTree>) -> bool {
        matches!(tree, Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Bracket)
    }

    fn add_tree(&mut self, tree: &TokenTree) {
        match tree {
            TokenTree::Group(group) => self.add_group(group),
            TokenTree::Ident(ident) => self.push(ident.to_string(), ident.span(), TokenKind::Name),
            TokenTree::Punct(punct) => {
                self.push(punct.as_char().to_string(), punct.span(), TokenKind::Symbol);
            }
            TokenTree::Literal(literal) => {
                self.push(literal.to_string(), literal.span(), TokenKind::Literal);
            }
        }
    }

    fn add_group(&mut self, group: &Group) {
        let (open, close) = Self::delimiters(group.delimiter());
        self.push(open.to_owned(), group.span_open(), TokenKind::Symbol);
        self.add_stream(group.stream());
        self.push(close.to_owned(), group.span_close(), TokenKind::Symbol);
    }

    fn delimiters(delimiter: Delimiter) -> (&'static str, &'static str) {
        match delimiter {
            Delimiter::Parenthesis => ("(", ")"),
            Delimiter::Brace => ("{", "}"),
            Delimiter::Bracket => ("[", "]"),
            Delimiter::None => ("", ""),
        }
    }

    fn push(&mut self, text: String, span: Span, kind: TokenKind) {
        self.tokens
            .push(CodeToken::new(text, span.start().line, kind));
    }
}
