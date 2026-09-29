#![allow(clippy::expect_used)]

use super::java_tests::{java_web_ready, run_driver, write};
use super::{BuildJobs, WebLanguage, build_all};
use std::path::Path;

struct JavaBehaviorFixtures;

impl JavaBehaviorFixtures {
    const FILES: [(&str, &str); 7] = [
        (
            "client/services/OrdersService.java",
            r#"@Service
class OrdersService {
    Behaviors.Context behaviors;
    private static final int FIRST_WORKING_ATTEMPT = 3;
    private int attempts = 0;

    @OnStart
    void open() {
        behaviors = Behaviors.create("OrdersSetup", new Behaviors.Options("orders", new Behaviors.Signal()));
    }

    @Retry({"maxRetries":3,"delayMs":10,"multiplier":2,"maxDelayMs":200,"jitterMs":5})
    Object submit(String id) {
        attempts++;
        failFirstTwoAttempts();
        return reportOf(id);
    }

    @MaxAtOnce({"limit":2,"policy":"block"})
    Object report(String id) {
        return reportOf(id);
    }

    private Object reportOf(String id) {
        return java.util.Map.of("id", id, "attempts", attempts);
    }

    private void failFirstTwoAttempts() {
        if (attempts < FIRST_WORKING_ATTEMPT) {
            throw new IllegalStateException("the checkout API is unavailable");
        }
    }
}
"#,
        ),
        (
            "client/setups/OrdersSetup.java",
            r#"@Setup
class OrdersSetup {
    static Behaviors.Configuration configure(Behaviors.Execution execution) {
        return new Behaviors.Configuration(java.util.Map.of(), java.util.Map.of(), java.util.Map.of(),
            (call, current) -> true);
    }
}
"#,
        ),
        (
            "client/services/SettingsService.java",
            r#"@Service
class SettingsService {
    String title = "Catalog";
    long pageSize = 20;

    @OnStart
    void bind() {
        java.util.Map<String, Object> publicCatalog = java.util.Map.of(
            "title", "Catalog",
            "page", java.util.Map.of("size", 20),
            "cache", java.util.Map.of("enabled", true));
        java.util.Map<String, Object> published = java.util.Map.of("tac", java.util.Map.of("catalog", publicCatalog));
        String setup = "CatalogSetup";
        java.util.Map<String, Object> catalog = Behaviors.properties(setup, published);
        java.util.Map<String, Object> values = Behaviors.values("PublicValues", published);
        title = (String) values.get("title");
        pageSize = ((Number) catalog.get("pageSize")).longValue();
        Behaviors.Options options = new Behaviors.Options(
            "public-catalog", new Behaviors.Signal(), 10000, java.util.List.of("public"), published);
        Behaviors.create(setup, options).close();
    }
}
"#,
        ),
        (
            "client/setups/CatalogSetup.java",
            r#"@Setup
@Settings({"prefix":"TAC_CATALOG_"})
@WhenProfile("public")
@WhenSetting({"name":"TAC_CATALOG_CACHE_ENABLED","havingValue":"true","matchIfMissing":false})
class CatalogSetup {
    @Min(1)
    @Max(100)
    int pageSize = 20;
    boolean cacheEnabled = false;
    String title = "Catalog";
    static Behaviors.Configuration configure(Behaviors.Execution execution) {
        return new Behaviors.Configuration();
    }
}
"#,
        ),
        (
            "client/helpers/PublicValues.java",
            r#"class PublicValues {
    @Setting("${tac.catalog.title:Catalog}")
    String title = "";
}
"#,
        ),
        (
            "client/services/OrderService.java",
            r#"@Service
class OrderService {
    Object accept(@Check(OrderInput.class) Object order) {
        return order;
    }
}
"#,
        ),
        (
            "client/helpers/OrderInput.java",
            r#"@Checked
class OrderInput {
    @NotEmpty
    @Length({"min":1,"max":80})
    String name;
    @Required
    @Min(1)
    @Max(9)
    Object level;
}
"#,
        ),
    ];

    const SERVICES: [(&str, &str); 3] = [
        ("orders", "OrdersService"),
        ("settings", "SettingsService"),
        ("order", "OrderService"),
    ];

    const DRIVER: &str = r"import { readFile } from 'node:fs/promises'
globalThis.fetch = async (url) => new Response(await readFile(url))
const { default: OrdersService } = await import('./OrdersService.js')
const { default: SettingsService } = await import('./SettingsService.js')
const { default: OrderService } = await import('./OrderService.js')
const host = globalThis[Symbol.for('tachyon.java.host')]
const pauses = []
const pause = host.pause
host.pause = (milliseconds) => { pauses.push(milliseconds); pause(milliseconds) }
const orders = new OrdersService({})
orders.__tachyonStarted()
const started = performance.now()
const submitted = orders.submit('A1')
const waited = performance.now() - started
const reported = orders.report('A1')
const settings = new SettingsService({})
settings.__tachyonStarted()
const order = new OrderService({})
const accepted = order.accept({ name: 'Tea', level: 2 })
let refused = ''
try { order.accept({ name: '', level: 12 }) } catch (error) { refused = error.message }
console.log(JSON.stringify({ submitted, waited, pauses: pauses.length, reported, title: settings.title, pageSize: settings.pageSize, accepted, refused }))
process.exit(0)
";
}

async fn publish(root: &Path) {
    let services: Vec<_> = JavaBehaviorFixtures::SERVICES
        .iter()
        .map(|(instance, class)| {
            (
                String::from(*instance),
                String::from(*class),
                WebLanguage::Java,
                root.join(format!("client/services/{class}.java")),
            )
        })
        .collect();
    let built = build_all(
        root,
        &services,
        &BuildJobs::new(1),
        crate::BuildProfile::Development,
    )
    .await
    .expect("the Java services build");
    for (name, bytes) in built.files {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        std::fs::write(path, bytes).expect("publish");
    }
}

#[tokio::test]
async fn a_java_service_retries_after_a_host_clock_wait_and_binds_and_checks_values() {
    if !java_web_ready() {
        eprintln!("skipping: TeaVM (TAC_TEAVM_HOME), javac or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    for (relative, text) in JavaBehaviorFixtures::FILES {
        write(root.path(), relative, text);
    }
    publish(root.path()).await;
    let report = run_driver(root.path(), JavaBehaviorFixtures::DRIVER);
    assert_eq!(
        report["submitted"],
        serde_json::json!({"id": "A1", "attempts": 3})
    );
    assert_eq!(report["pauses"], 2, "{report}");
    assert!(
        report["waited"].as_f64().expect("waited") >= 30.0,
        "{report}"
    );
    assert_eq!(
        report["reported"],
        serde_json::json!({"id": "A1", "attempts": 3})
    );
    assert_eq!(report["title"], "Catalog");
    assert_eq!(report["pageSize"], 20);
    assert_eq!(
        report["accepted"],
        serde_json::json!({"name": "Tea", "level": 2})
    );
    assert!(
        report["refused"]
            .as_str()
            .is_some_and(|text| !text.is_empty()),
        "{report}"
    );
}

struct TextCheck;

impl TextCheck {
    const PROGRAM: &str = r#"public class TextCheck {
    static int wrong = 0;

    static void same(String sample, boolean found, String rule) {
        if (found != sample.matches(rule)) {
            wrong++;
            System.out.println("different: " + rule + " " + sample);
        }
    }

    static void check(String sample) {
        same(sample, TacText.profileName(sample), "[A-Za-z_$][A-Za-z0-9_$]{0,63}");
        same(sample, TacText.pathSegment(sample), "[A-Za-z0-9._~-]{1,256}");
        same(sample, TacText.settingName(sample), "[A-Z][A-Z0-9_]*");
        same(sample, TacText.lowerWord(sample), "[a-z][a-z0-9]*");
        same(sample, TacText.integerText(sample), "-?(0|[1-9][0-9]*)");
        same(sample, TacText.numberText(sample), "-?(0|[1-9][0-9]*)(\\.[0-9]+)?([eE][+-]?[0-9]+)?");
        same(sample, TacText.headerName(sample), "[!#$%&'*+.^_`|~0-9A-Za-z-]{1,128}");
        for (char separator : new char[] {'/', ':'}) {
            String[] expected = sample.split(java.util.regex.Pattern.quote(String.valueOf(separator)), -1);
            if (!java.util.Arrays.equals(expected, TacText.split(sample, separator))) {
                wrong++;
                System.out.println("different split: " + sample);
            }
        }
    }

    public static void main(String[] arguments) {
        String alphabet = "aZ09_$-.+eE:/~!`";
        java.util.List<String> samples = new java.util.ArrayList<>(java.util.List.of(""));
        for (int length = 1; length <= 4; length++) {
            java.util.List<String> longer = new java.util.ArrayList<>();
            for (String sample : samples) {
                if (sample.length() == length - 1) {
                    for (char character : alphabet.toCharArray()) {
                        longer.add(sample + character);
                    }
                }
            }
            samples.addAll(longer);
        }
        for (int length : new int[] {63, 64, 65, 127, 128, 129, 255, 256, 257}) {
            samples.add("a".repeat(length));
            samples.add("A".repeat(length));
            samples.add("1".repeat(length));
        }
        samples.addAll(java.util.List.of("-0.5E-3", "12.25e+10", "1e", "1.", ".5", "-", "007", "a b", "é"));
        for (String sample : samples) {
            check(sample);
        }
        for (int character = 0; character < 32; character++) {
            if (!String.format(java.util.Locale.ROOT, "\\u%04x", character).equals(TacText.unicodeEscape(character))) {
                wrong++;
                System.out.println("different escape: " + character);
            }
        }
        System.out.println("checked " + samples.size() + " wrong " + wrong);
    }
}
"#;
}

#[test]
fn the_regex_free_text_checks_answer_like_the_regular_expressions() {
    let runs = |program: &str| {
        std::process::Command::new(program)
            .arg("-version")
            .output()
            .is_ok_and(|output| output.status.success())
    };
    if !runs("javac") || !runs("java") {
        eprintln!("skipping: javac or java is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("folder");
    write(
        root.path(),
        "TacText.java",
        include_str!("../native/java_web_services/prelude/TacText.java"),
    );
    write(root.path(), "TextCheck.java", TextCheck::PROGRAM);
    let compiled = std::process::Command::new("javac")
        .args(["-d", ".", "TacText.java", "TextCheck.java"])
        .current_dir(root.path())
        .status()
        .expect("javac");
    assert!(compiled.success());
    let output = std::process::Command::new("java")
        .args(["-cp", ".", "TextCheck"])
        .current_dir(root.path())
        .output()
        .expect("java");
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.trim_end().ends_with("wrong 0"), "{text}");
}
