use super::error_idiom::ErrorIdiom;
use super::go_error_return::GoErrorReturn;
use super::kotlin_elvis_return::KotlinElvisReturn;
use super::swift_guard_return::SwiftGuardReturn;
use super::words::Words;

pub(super) struct ErrorReturn<'r, 'a> {
    words: &'r Words<'a>,
}

impl<'r, 'a> ErrorReturn<'r, 'a> {
    pub(super) fn new(words: &'r Words<'a>) -> Self {
        Self { words }
    }

    pub(super) fn matches(&self, idiom: ErrorIdiom, at: usize) -> bool {
        self.idiom_matches(idiom, at)
    }

    fn idiom_matches(&self, idiom: ErrorIdiom, at: usize) -> bool {
        match idiom {
            ErrorIdiom::GoErrorCheck => GoErrorReturn::new(self.words).matches(at),
            ErrorIdiom::SwiftGuard => SwiftGuardReturn::new(self.words).matches(at),
            ErrorIdiom::KotlinElvis => KotlinElvisReturn::new(self.words).matches(at),
            ErrorIdiom::None => false,
        }
    }
}
