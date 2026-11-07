pub(crate) struct GroovySource;

impl GroovySource {
    pub(crate) const EXTENSION: &'static str = "groovy";
    const SYNTAX: &'static str = "java";
    const JAVA_PART: &'static str = "\n//__YON_JAVA__\n";

    pub(crate) fn syntax(extension: &str) -> &str {
        Self::syntax_of(extension)
    }

    pub(crate) fn is_groovy(path: &std::path::Path) -> bool {
        path.extension()
            .is_some_and(|extension| extension == Self::EXTENSION)
    }

    pub(crate) fn with_java_part(extension: &str, contents: &str) -> Option<String> {
        let needed = Self::needs_java_part(extension, contents);
        needed.then(|| format!("{contents}{}", Self::JAVA_PART))
    }

    pub(crate) fn before_java_part(program: &str, inserted: &str) -> String {
        let at = Self::java_part_start(program);
        format!("{}\n{inserted}{}", &program[..at], &program[at..])
    }

    pub(crate) fn split(program: &str) -> (&str, &str) {
        let at = Self::java_part_start(program);
        let java_start = (at + Self::JAVA_PART.len()).min(program.len());
        (&program[..at], &program[java_start..])
    }

    fn syntax_of(extension: &str) -> &str {
        if extension == Self::EXTENSION {
            return Self::SYNTAX;
        }
        extension
    }

    fn needs_java_part(extension: &str, contents: &str) -> bool {
        extension == Self::EXTENSION && !contents.contains(Self::JAVA_PART)
    }

    fn java_part_start(program: &str) -> usize {
        program.find(Self::JAVA_PART).unwrap_or(program.len())
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
