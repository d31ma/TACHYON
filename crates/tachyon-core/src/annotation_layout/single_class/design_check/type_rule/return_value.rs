use super::super::super::cohesion::Method;
use super::super::class_file::ClassFile;

pub(super) struct ReturnValue;

impl ReturnValue {
    const RETURN: &'static str = "return";
    const YIELD: &'static str = "yield";
    const EMPTY_ENDS: [&'static str; 2] = [";", "}"];
    const CLOSE: &'static str = ")";

    pub(super) fn gives_value(class: &ClassFile, method: &Method) -> bool {
        method
            .body
            .clone()
            .any(|at| Self::gives_value_at(class, method, at))
    }

    pub(super) fn closing_parenthesis(class: &ClassFile, close: usize) -> Option<usize> {
        let found = class.words.text(close) == Self::CLOSE;
        found.then(|| class.words.start(close))
    }

    fn gives_value_at(class: &ClassFile, method: &Method, at: usize) -> bool {
        Self::is_value_word(class, at) & method.owns(at)
    }

    fn is_value_word(class: &ClassFile, at: usize) -> bool {
        class.words.text(at) == Self::YIELD || Self::is_valued_return(class, at)
    }

    fn is_valued_return(class: &ClassFile, at: usize) -> bool {
        let next = at + 1;
        (class.words.text(at) == Self::RETURN)
            & !class.words.line_start(next)
            & !Self::EMPTY_ENDS.contains(&class.words.text(next))
    }
}
