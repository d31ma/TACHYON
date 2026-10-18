use crate::Failure;
use std::path::Path;

#[derive(Default)]
pub(super) struct SwiftDeclarations {
    finished: Vec<String>,
    current: String,
    depth: usize,
    opened: bool,
}

impl SwiftDeclarations {
    pub(super) fn of(identity: &Path, source: &str) -> Result<Vec<String>, Failure> {
        let masked = crate::annotation_layout::masked_source(identity, source)?;
        let declarations = source
            .split_inclusive('\n')
            .zip(masked.split_inclusive('\n'))
            .fold(Self::default(), |declarations, (line, code)| {
                declarations.with_line(line, code)
            });
        Ok(declarations.finished())
    }

    fn with_line(mut self, line: &str, code: &str) -> Self {
        self.current.push_str(line);
        self.opened |= code.contains('{');
        self.depth = crate::annotation_layout::brace_depth_after(code, self.depth);
        if [self.depth == 0, self.opened] == [true, true] {
            self.finished.push(std::mem::take(&mut self.current));
            self.opened = false;
        }
        self
    }

    fn finished(mut self) -> Vec<String> {
        let rest = !self.current.trim().is_empty();
        self.finished.extend(rest.then_some(self.current));
        self.finished
    }
}
