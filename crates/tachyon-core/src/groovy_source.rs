pub(crate) struct GroovySource;

impl GroovySource {
    pub(crate) const EXTENSION: &'static str = "groovy";
    const SYNTAX: &'static str = "java";
    const JAVA_PART: &'static str = "\n//__YON_JAVA__\n";

    pub(crate) fn syntax(extension: &str) -> &str {
        if extension == Self::EXTENSION {
            return Self::SYNTAX;
        }
        extension
    }

    pub(crate) fn is_groovy(path: &std::path::Path) -> bool {
        path.extension()
            .is_some_and(|extension| extension == Self::EXTENSION)
    }

    pub(crate) fn with_java_part(extension: &str, contents: &str) -> Option<String> {
        if extension != Self::EXTENSION || contents.contains(Self::JAVA_PART) {
            return None;
        }
        Some(format!("{contents}{}", Self::JAVA_PART))
    }

    pub(crate) fn before_java_part(program: &str, inserted: &str) -> String {
        match program.find(Self::JAVA_PART) {
            Some(at) => format!("{}\n{inserted}{}", &program[..at], &program[at..]),
            None => format!("{program}\n{inserted}"),
        }
    }

    pub(crate) fn split(program: &str) -> (&str, &str) {
        match program.find(Self::JAVA_PART) {
            Some(at) => (&program[..at], &program[at + Self::JAVA_PART.len()..]),
            None => (program, ""),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::GroovySource;

    #[test]
    fn groovy_reads_with_the_java_syntax_and_other_languages_keep_theirs() {
        assert_eq!(GroovySource::syntax("groovy"), "java");
        assert_eq!(GroovySource::syntax("kt"), "kt");
    }

    #[test]
    fn the_generated_java_part_follows_the_authored_groovy_part() {
        let marked = GroovySource::with_java_part("groovy", "class A {}").unwrap_or_default();
        let program = format!("{marked}final class Handler {{}}\n");
        let program = GroovySource::before_java_part(&program, "record Order(String name) {}\n");
        let (groovy, java) = GroovySource::split(&program);
        assert!(groovy.starts_with("class A {}") && groovy.contains("record Order"));
        assert_eq!(java, "final class Handler {}\n");
        assert!(GroovySource::with_java_part("java", "class A {}").is_none());
    }
}
