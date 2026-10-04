use super::{CppClass, CppFailure, CppFileReader, CppSource, CppTokens};
use crate::Failure;
use std::ops::Range;
use std::path::Path;

pub(crate) struct CppFile {
    pub(crate) class: CppClass,
    pub(crate) directives: Vec<Range<usize>>,
}

impl CppFile {
    pub(crate) fn read(path: &Path, source: &str) -> Result<Self, Failure> {
        let read =
            CppSource::read(source).map_err(|message| CppFailure::unreadable(path, 0, message))?;
        let tokens = CppTokens::new(read.tokens);
        let class = CppFileReader::only_class(path, source, &tokens)?;
        Ok(Self {
            class,
            directives: read.directives,
        })
    }
}
