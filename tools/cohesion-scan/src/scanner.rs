use crate::function::{Construct, Function};
use proc_macro2::Span;
use quote::ToTokens;
use std::collections::HashSet;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{
    Attribute, BinOp, Expr, ExprBinary, ExprCall, ExprForLoop, ExprIf, ExprLoop, ExprMatch,
    ExprMethodCall, ExprWhile, File, Ident, ImplItemFn, ItemFn, ItemImpl, ItemMod, ItemTrait,
    Local, Macro, PatGuard, Token, TraitItemFn, Visibility,
};

type Position = (usize, usize);

pub(crate) struct FunctionScanner {
    finished: Vec<Function>,
    open: Vec<Function>,
    public_scopes: Vec<bool>,
    absorbed: HashSet<Position>,
    else_ifs: HashSet<Position>,
}

impl FunctionScanner {
    const TEST_MARKS: [&'static str; 3] = ["test", "tokio::test", "cfg(test)"];
    const SHORT_CIRCUITS: [&'static str; 2] = ["&&", "||"];
    const MATCH_MACROS: [&'static str; 2] = ["matches", "select"];
    const ITERATOR_METHODS: [&'static str; 34] = [
        "all",
        "any",
        "chunk_by",
        "dedup_by",
        "dedup_by_key",
        "filter",
        "filter_map",
        "find",
        "find_map",
        "flat_map",
        "fold",
        "for_each",
        "inspect",
        "is_sorted_by",
        "map",
        "map_while",
        "max_by",
        "max_by_key",
        "min_by",
        "min_by_key",
        "partition",
        "position",
        "reduce",
        "retain",
        "retain_mut",
        "rfind",
        "rposition",
        "scan",
        "skip_while",
        "sort_by",
        "sort_by_key",
        "take_while",
        "try_fold",
        "try_for_each",
    ];

    pub(crate) fn functions(file: &File) -> Vec<Function> {
        let mut scanner = Self {
            finished: Vec::new(),
            open: vec![Function::new(String::new(), 0, false)],
            public_scopes: Vec::new(),
            absorbed: HashSet::new(),
            else_ifs: HashSet::new(),
        };
        scanner.visit_file(file);
        scanner.finished
    }

    fn position(span: Span) -> Position {
        let start = span.start();
        (start.line, start.column)
    }

    fn text(tokens: &impl ToTokens) -> String {
        tokens.to_token_stream().to_string().replace(' ', "")
    }

    fn is_test_attribute(attribute: &Attribute) -> bool {
        let texts = [Self::text(attribute.path()), Self::text(&attribute.meta)];
        texts
            .iter()
            .any(|text| Self::TEST_MARKS.contains(&text.as_str()))
    }

    fn is_public(visibility: &Visibility) -> bool {
        !matches!(visibility, Visibility::Inherited)
    }

    fn is_short_circuit(operator: &BinOp) -> bool {
        Self::SHORT_CIRCUITS.contains(&Self::text(operator).as_str())
    }

    fn last_name(path: &syn::Path) -> String {
        path.segments
            .last()
            .map(|segment| segment.ident.to_string())
            .unwrap_or_default()
    }

    fn current(&mut self) -> &mut Function {
        let index = self.open.len() - 1;
        &mut self.open[index]
    }

    fn scope_is_public(&self) -> bool {
        self.public_scopes.last().copied().unwrap_or(false)
    }

    fn record(&mut self, construct: Construct, span: Span) {
        let line = span.start().line;
        self.current().add_construct(construct, line);
    }

    fn remember_call(&mut self, name: &Ident) {
        self.current().add_call(name.to_string());
    }

    fn absorb(&mut self, expression: &Expr) {
        self.absorbed.insert(Self::position(expression.span()));
    }

    fn construct_count(&mut self) -> usize {
        self.current().construct_count()
    }

    fn visit_owned(&mut self, expression: &Expr) {
        Visit::visit_expr(self, expression);
    }

    fn enter_function(&mut self, function: Function, walk: impl FnOnce(&mut Self)) {
        self.open.push(function);
        walk(self);
        let finished = self.open.pop().map(Function::without_self_calls);
        self.finished.extend(finished);
    }

    fn enter_scope(&mut self, public: bool, walk: impl FnOnce(&mut Self)) {
        self.public_scopes.push(public);
        walk(self);
        self.public_scopes.pop();
    }

    fn unless_test(&mut self, attributes: &[Attribute], walk: impl FnOnce(&mut Self)) {
        if attributes.iter().any(Self::is_test_attribute) {
            return;
        }
        walk(self);
    }

    fn remember_else_if(&mut self, node: &ExprIf) {
        if let Some((_, otherwise)) = &node.else_branch
            && let Expr::If(inner) = otherwise.as_ref()
        {
            self.else_ifs.insert(Self::position(inner.if_token.span));
        }
    }

    fn record_if(&mut self, node: &ExprIf) {
        if self.else_ifs.contains(&Self::position(node.if_token.span)) {
            return;
        }
        self.record(Construct::If, node.if_token.span);
    }

    fn record_let_else(&mut self, node: &Local) {
        if let Some(init) = &node.init
            && init.diverge.is_some()
        {
            self.record(Construct::LetElse, node.let_token.span);
        }
    }

    fn record_short_circuit(&mut self, node: &ExprBinary) {
        let absorbed = self.absorbed.contains(&Self::position(node.span()));
        if !Self::is_short_circuit(&node.op) || absorbed {
            return;
        }
        self.record(Construct::ShortCircuit, node.op.span());
    }

    fn absorb_chain(&mut self, node: &ExprBinary) {
        if Self::is_short_circuit(&node.op) {
            self.absorb(&node.left);
            self.absorb(&node.right);
        }
    }

    fn record_iterator_logic(&mut self, node: &ExprMethodCall, count_before: usize) {
        let method = node.method.to_string();
        let iterator = Self::ITERATOR_METHODS.contains(&method.as_str());
        if !iterator || self.construct_count() == count_before {
            return;
        }
        self.record(Construct::IteratorLogic, node.method.span());
    }

    fn remember_path_call(&mut self, function: &Expr) {
        if let Expr::Path(path) = function
            && let Some(segment) = path.path.segments.last()
        {
            self.remember_call(&segment.ident);
        }
    }

    fn record_macro_match(&mut self, node: &Macro) {
        if Self::MATCH_MACROS.contains(&Self::last_name(&node.path).as_str()) {
            self.record(Construct::Match, node.path.span());
        }
    }

    fn visit_macro_arguments(&mut self, node: &Macro) {
        node.parse_body_with(Punctuated::<Expr, Token![,]>::parse_terminated)
            .into_iter()
            .flatten()
            .for_each(|argument| self.visit_owned(&argument));
    }
}

impl<'ast> Visit<'ast> for FunctionScanner {
    fn visit_item_mod(&mut self, node: &'ast ItemMod) {
        self.unless_test(&node.attrs, |scanner| visit::visit_item_mod(scanner, node));
    }

    fn visit_item_impl(&mut self, node: &'ast ItemImpl) {
        let public = node.trait_.is_some();
        self.unless_test(&node.attrs, |scanner| {
            scanner.enter_scope(public, |inner| visit::visit_item_impl(inner, node));
        });
    }

    fn visit_item_trait(&mut self, node: &'ast ItemTrait) {
        let public = Self::is_public(&node.vis);
        self.enter_scope(public, |scanner| visit::visit_item_trait(scanner, node));
    }

    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        let ident = &node.sig.ident;
        let line = ident.span().start().line;
        let function = Function::new(ident.to_string(), line, Self::is_public(&node.vis));
        self.unless_test(&node.attrs, |scanner| {
            scanner.enter_function(function, |inner| visit::visit_item_fn(inner, node));
        });
    }

    fn visit_impl_item_fn(&mut self, node: &'ast ImplItemFn) {
        let ident = &node.sig.ident;
        let line = ident.span().start().line;
        let public = [Self::is_public(&node.vis), self.scope_is_public()].contains(&true);
        let function = Function::new(ident.to_string(), line, public);
        self.unless_test(&node.attrs, |scanner| {
            scanner.enter_function(function, |inner| visit::visit_impl_item_fn(inner, node));
        });
    }

    fn visit_trait_item_fn(&mut self, node: &'ast TraitItemFn) {
        let ident = &node.sig.ident;
        let line = ident.span().start().line;
        let function = Function::new(ident.to_string(), line, self.scope_is_public());
        self.unless_test(&node.attrs, |scanner| {
            scanner.enter_function(function, |inner| visit::visit_trait_item_fn(inner, node));
        });
    }

    fn visit_expr_if(&mut self, node: &'ast ExprIf) {
        self.remember_else_if(node);
        self.record_if(node);
        self.absorb(&node.cond);
        visit::visit_expr_if(self, node);
    }

    fn visit_expr_match(&mut self, node: &'ast ExprMatch) {
        self.record(Construct::Match, node.match_token.span);
        visit::visit_expr_match(self, node);
    }

    fn visit_pat_guard(&mut self, node: &'ast PatGuard) {
        self.absorb(&node.guard);
        visit::visit_pat_guard(self, node);
    }

    fn visit_expr_for_loop(&mut self, node: &'ast ExprForLoop) {
        self.record(Construct::Loop, node.for_token.span);
        visit::visit_expr_for_loop(self, node);
    }

    fn visit_expr_while(&mut self, node: &'ast ExprWhile) {
        self.record(Construct::Loop, node.while_token.span);
        self.absorb(&node.cond);
        visit::visit_expr_while(self, node);
    }

    fn visit_expr_loop(&mut self, node: &'ast ExprLoop) {
        self.record(Construct::Loop, node.loop_token.span);
        visit::visit_expr_loop(self, node);
    }

    fn visit_local(&mut self, node: &'ast Local) {
        self.record_let_else(node);
        visit::visit_local(self, node);
    }

    fn visit_expr_binary(&mut self, node: &'ast ExprBinary) {
        self.record_short_circuit(node);
        self.absorb_chain(node);
        visit::visit_expr_binary(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        self.remember_path_call(&node.func);
        visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        self.remember_call(&node.method);
        self.visit_expr(&node.receiver);
        let count_before = self.construct_count();
        node.args
            .iter()
            .for_each(|argument| self.visit_expr(argument));
        self.record_iterator_logic(node, count_before);
    }

    fn visit_macro(&mut self, node: &'ast Macro) {
        self.record_macro_match(node);
        self.visit_macro_arguments(node);
    }
}
