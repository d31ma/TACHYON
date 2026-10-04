use super::{CppAttributes, CppClass, CppFailure, CppMemberReader, CppSource, CppTokens};
use crate::Failure;
use std::ops::Range;
use std::path::Path;

pub(crate) struct CppFile {
    pub(crate) class: CppClass,
    pub(crate) directives: Vec<Range<usize>>,
}

struct CppFileReader<'a> {
    path: &'a Path,
    source: &'a str,
    tokens: CppTokens,
    classes: Vec<CppClass>,
}

impl CppFile {
    pub(crate) fn read(path: &Path, source: &str) -> Result<Self, Failure> {
        let read =
            CppSource::read(source).map_err(|message| CppFailure::unreadable(path, 0, message))?;
        let mut reader = CppFileReader {
            path,
            source,
            tokens: CppTokens::new(read.tokens),
            classes: Vec::new(),
        };
        reader.top_level()?;
        let class = reader.only_class()?;
        Ok(Self {
            class,
            directives: read.directives,
        })
    }
}

impl CppFileReader<'_> {
    const TYPE_WORDS: [&'static str; 2] = ["class", "struct"];
    const LOWERED_LATER: &'static str =
        "C++ Tac files declare classes without templates or namespaces";
    const OUTSIDE: &'static str = "A C++ class file holds one class and its #include lines; this declaration is outside the class. Move it into the class or into its own file";
    const SECOND: &'static str = "A C++ class file holds one class (ADR 0067); move the second class into its own file named after it";
    const NONE: &'static str = "A C++ class file declares one class or enum named after the file";
    const LEADING: &'static str = "C++ puts a class attribute after the class key: write `class [[Service]] OrderService`, not `[[Service]] class OrderService`";

    fn top_level(&mut self) -> Result<(), Failure> {
        let mut at = 0;
        while at < self.tokens.len() {
            at = self.declaration(at)?;
        }
        Ok(())
    }

    fn failure(&self, index: usize, message: &str) -> Failure {
        CppFailure::unreadable(
            self.path,
            self.tokens.start(index).min(self.source.len()),
            message,
        )
    }

    fn declaration(&mut self, at: usize) -> Result<usize, Failure> {
        match self.tokens.text(at) {
            ";" => Ok(at + 1),
            "using" => Ok(self.statement_end(at)),
            "class" | "struct" => self.class(at),
            "enum" => self.enumeration(at),
            "template" | "namespace" => Err(self.failure(at, Self::LOWERED_LATER)),
            "[" if self.tokens.opens_attribute(at) => Err(self.failure(at, Self::LEADING)),
            _ => Err(self.failure(at, Self::OUTSIDE)),
        }
    }

    fn statement_end(&self, at: usize) -> usize {
        (at..self.tokens.len())
            .find(|index| self.tokens.is(*index, ";"))
            .map_or(self.tokens.len(), |index| index + 1)
    }

    fn class(&mut self, keyword: usize) -> Result<usize, Failure> {
        let (markers, name) = CppAttributes::read(&self.tokens, self.source, keyword + 1);
        if self.tokens.is(name + 1, ";") {
            return Ok(name + 2);
        }
        let open = (name..self.tokens.len())
            .find(|index| self.tokens.is(*index, "{") || self.tokens.is(*index, ";"))
            .filter(|index| self.tokens.is(*index, "{"))
            .ok_or_else(|| self.failure(keyword, Self::NONE))?;
        let close = self
            .tokens
            .partner(open)
            .ok_or_else(|| self.failure(open, Self::NONE))?;
        let public = self.tokens.is(keyword, "struct");
        let class_name = self.tokens.text(name).to_owned();
        let members = CppMemberReader::read(
            &self.tokens,
            self.source,
            &class_name,
            public,
            open + 1..close,
        );
        self.add(
            keyword,
            CppClass {
                name: class_name,
                name_at: self.tokens.start(name),
                markers,
                members,
                enumerators: Vec::new(),
                is_enum: false,
                body: self.tokens.start(open)..self.tokens.end(close),
            },
        )?;
        Ok(close + 1)
    }

    fn enumeration(&mut self, keyword: usize) -> Result<usize, Failure> {
        let scoped = Self::TYPE_WORDS.contains(&self.tokens.text(keyword + 1));
        let (markers, name) =
            CppAttributes::read(&self.tokens, self.source, keyword + 1 + usize::from(scoped));
        let open = (name..self.tokens.len())
            .find(|index| self.tokens.is(*index, "{"))
            .ok_or_else(|| self.failure(keyword, Self::NONE))?;
        let close = self
            .tokens
            .partner(open)
            .ok_or_else(|| self.failure(open, Self::NONE))?;
        let enumerators = (open + 1..close)
            .filter(|index| self.tokens.list[*index].is_word())
            .filter(|index| self.tokens.is(index - 1, "{") || self.tokens.is(index - 1, ","))
            .map(|index| self.tokens.text(index).to_owned())
            .collect();
        self.add(
            keyword,
            CppClass {
                name: self.tokens.text(name).to_owned(),
                name_at: self.tokens.start(name),
                markers,
                members: Vec::new(),
                enumerators,
                is_enum: true,
                body: self.tokens.start(open)..self.tokens.end(close),
            },
        )?;
        Ok(close + 1)
    }

    fn add(&mut self, keyword: usize, class: CppClass) -> Result<(), Failure> {
        if !self.classes.is_empty() {
            return Err(self.failure(keyword, Self::SECOND));
        }
        self.classes.push(class);
        Ok(())
    }

    fn only_class(&mut self) -> Result<CppClass, Failure> {
        self.classes
            .pop()
            .ok_or_else(|| CppFailure::unreadable(self.path, 0, Self::NONE))
    }
}
