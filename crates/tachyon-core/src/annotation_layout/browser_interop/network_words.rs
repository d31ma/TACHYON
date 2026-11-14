pub(super) struct NetworkWords {
    pub(super) words: &'static [&'static str],
    pub(super) imports: &'static [&'static str],
}

impl NetworkWords {
    pub(super) const NONE: Self = Self::words(&[]);

    pub(super) const fn words(words: &'static [&'static str]) -> Self {
        Self {
            words,
            imports: &[],
        }
    }

    pub(super) const fn imports(imports: &'static [&'static str]) -> Self {
        Self {
            words: &[],
            imports,
        }
    }
}
