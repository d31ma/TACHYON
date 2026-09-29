use super::Module;

pub(super) enum Prepared {
    Erased(String),
    Annotated(Module),
}
