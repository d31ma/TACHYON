use super::SecretCheck;
use std::path::Path;

fn refusals(path: &str, source: &str) -> Vec<String> {
    let files = [(Path::new(path), source.as_bytes())];
    SecretCheck::diagnostics(files.into_iter())
        .iter()
        .map(|diagnostic| diagnostic.message.clone())
        .collect()
}

#[test]
fn known_token_shapes_are_refused_in_every_kind_of_file() {
    for (path, source, kind) in [
        (
            "server/services/MailService.js",
            "const key = 'AKIAQWERTYUIOPASDFGH';\n",
            "an AWS access key",
        ),
        (
            "server/services/Git.py",
            "TOKEN = \"ghp_0123456789abcdefghijABCDEFGHIJ012345\"\n",
            "a GitHub token",
        ),
        (
            "server/services/Pay.java",
            "String key = \"sk_live_4eC39HqLyjWDarjtT1zdp7dc\";\n",
            "a Stripe secret key",
        ),
        (
            "client/pages/tac.html",
            "<p>xoxb-123456789012-1234567890123-abc</p>\n",
            "a Slack token",
        ),
        (
            "server/application.json",
            "{\"map\": \"AIzaSyA1234567890abcdefghijklmnopqrstuv\"}\n",
            "a Google API key",
        ),
        (
            "server/services/Keys.rs",
            "const KEY: &str = \"-----BEGIN RSA PRIVATE KEY-----\";\n",
            "a private key",
        ),
        (
            "server/services/Auth.go",
            "t := \"eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U\"\n",
            "a JSON Web Token",
        ),
        (
            "server/services/Db.cs",
            "var clientSecret = \"q8Zr2Lk9Vx4Tn7Pw1Ys6\";\n",
            "a secret value in a variable with a secret name",
        ),
    ] {
        let found = refusals(path, source);
        assert_eq!(found.len(), 1, "{path}: {found:?}");
        assert!(found[0].contains(kind), "{found:?}");
        assert!(found[0].contains("line 1"), "{found:?}");
    }
}

#[test]
fn a_refusal_never_repeats_the_secret() {
    let found = refusals(
        "server/services/Git.ts",
        "const token = 'ghp_0123456789abcdefghijABCDEFGHIJ012345';\n",
    );
    assert!(!found[0].contains("ghp_0123"), "{found:?}");
}

#[test]
fn ordinary_code_is_not_a_secret() {
    for source in [
        "const tokenHeader = 'x-tachyon-request-id';\n",
        "password = settings.password\n",
        "const secretName = 'YON_DATABASE_PASSWORD';\n",
        "const tokenUrl = 'https://auth.example.com/oauth/token';\n",
        "SESSIONS = {\"ada-token\": Identity(\"ada\", [\"Admin\"])}\n",
        "const apiKey = `${prefix}${suffix}`;\n",
    ] {
        assert!(
            refusals("server/services/Plain.js", source).is_empty(),
            "{source}"
        );
    }
}

#[test]
fn only_marked_files_keep_values_that_say_they_are_examples() {
    let example = "const key = 'AKIAIOSFODNN7EXAMPLE';\n";
    assert_eq!(refusals("server/tests/KeyTest.js", example).len(), 1);
    let marked = format!("// tachyon: example secrets\n{example}");
    assert!(refusals("server/tests/KeyTest.js", &marked).is_empty());
    let real = "// tachyon: example secrets\nconst key = 'AKIAQWERTYUIOPASDFGH';\n";
    assert_eq!(refusals("server/tests/KeyTest.js", real).len(), 1);
}

#[test]
fn binary_files_are_not_read() {
    assert!(refusals("client/shared/assets/logo.png", "AKIAQWERTYUIOPASDFGH").is_empty());
}
