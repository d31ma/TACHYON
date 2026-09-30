use super::{Failure, Input, Parser, Path, Tokens};

pub(crate) struct SharedClass;

impl SharedClass {
    pub(crate) fn validate(path: &Path, source: &str) -> Result<(), Failure> {
        let input = Input::new(path, source);
        let tokens = Tokens::read(&input)?;
        tokens.check_complete(&input)?;
        Self::refuse_annotations(&input, &tokens)?;
        let mut parser = Parser::new(&input, &tokens);
        parser.read()?;
        parser.finish(None)?;
        Ok(())
    }

    fn refuse_annotations(input: &Input<'_>, tokens: &Tokens<'_>) -> Result<(), Failure> {
        if tokens.has_framework_marker() {
            return Err(input.failure(
                0,
                "Shared code cannot carry a Tachyon annotation",
                "Put an annotated class in its server role folder. Keep one plain class and its imports in server/shared/.",
            ));
        }
        Ok(())
    }
}
