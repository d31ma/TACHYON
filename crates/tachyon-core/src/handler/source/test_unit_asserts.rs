pub(super) struct TestUnitAsserts;

impl TestUnitAsserts {
    const TEST_FOLDER: &'static str = "server/tests/";
    const JAVA_FLAG: &'static str = "-ea";
    const DART_COMPILER: &'static str = "dart";
    const DART_FLAG: &'static str = "--enable-asserts";
    const PHP_SETTING: [&'static str; 2] = ["-d", "zend.assertions=1"];

    pub(super) fn java(portable: &str) -> Option<String> {
        Self::when_tested(portable, &[Self::JAVA_FLAG]).pop()
    }

    pub(super) fn dart(compiler: &str, portable: &str) -> Option<String> {
        Self::when_tested(portable, Self::dart_flags(compiler)).pop()
    }

    pub(super) fn php(portable: &str) -> Vec<String> {
        Self::when_tested(portable, &Self::PHP_SETTING)
    }

    fn dart_flags(compiler: &str) -> &'static [&'static str] {
        if compiler == Self::DART_COMPILER {
            return &[Self::DART_FLAG];
        }
        &[]
    }

    fn when_tested(portable: &str, flags: &[&str]) -> Vec<String> {
        if !portable.starts_with(Self::TEST_FOLDER) {
            return Vec::new();
        }
        flags.iter().map(|flag| (*flag).to_owned()).collect()
    }
}
