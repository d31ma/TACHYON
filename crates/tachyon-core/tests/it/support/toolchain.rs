pub struct Toolchain;

impl Toolchain {
    const INSTALLED_IN_CI: [&'static str; 13] = [
        "php", "php-cgi", "groovyc", "kotlinc", "dart", "lua", "go", "ruby", "bun", "cargo",
        "clang++", "dotnet", "javac",
    ];

    #[must_use]
    pub fn checked(program: &str, found: bool) -> bool {
        let required = [
            std::env::var_os("CI").is_some(),
            Self::INSTALLED_IN_CI.contains(&program),
        ] == [true, true];
        assert!(
            [required, found] != [true, false],
            "{program} is installed in CI, so this test must run there instead of skipping"
        );
        found
    }
}
