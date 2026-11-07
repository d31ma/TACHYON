use super::Input;
use crate::Failure;
use crate::groovy_source::GroovySource;

pub(super) struct JavaStreamReturn;

impl JavaStreamReturn {
    const MANY_VALUES: [&'static str; 2] = ["Stream", "Iterator"];
    const LANGUAGES: [&'static str; 2] = ["Java", "Groovy"];
    const UNMARKED: &'static str = "This method returns a Stream or an Iterator, so it answers with a stream. Mark it @StreamRoute and start the method name with 'stream', for example streamTicks";

    pub(super) fn check(
        input: &Input<'_>,
        start: usize,
        streaming: bool,
        returns: &str,
    ) -> Result<(), Failure> {
        let produces_many = Self::produces_many(returns);
        let problems = [
            (
                [streaming, produces_many] == [true, false],
                Self::wrong_return(input),
            ),
            (
                [streaming, produces_many] == [false, true],
                String::from(Self::UNMARKED),
            ),
        ];
        problems
            .into_iter()
            .find(|(present, _)| *present)
            .map_or(Ok(()), |(_, message)| Err(input.failure(start, message)))
    }

    fn produces_many(returns: &str) -> bool {
        let generic = returns.split('<').next().unwrap_or_default().trim();
        let kind = generic.rsplit('.').next().unwrap_or_default();
        Self::MANY_VALUES.contains(&kind)
    }

    fn wrong_return(input: &Input<'_>) -> String {
        let language = Self::LANGUAGES[usize::from(GroovySource::is_groovy(input.path))];
        format!(
            "A {language} @StreamRoute method returns java.util.stream.Stream<T> or java.util.Iterator<T>, so Tachyon can send each value as one event"
        )
    }
}
