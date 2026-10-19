#![allow(clippy::expect_used)]
use tachyon_contracts::{HandlerRequest, HttpMethod};
use tachyon_core::{
    HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions,
};

struct ProofLanguage {
    extension: &'static str,
    program: &'static str,
    flag: &'static str,
    names: [&'static str; 4],
    yon: bool,
    tac: bool,
    profiles: &'static str,
    settings: &'static str,
    setup: &'static str,
    unit: &'static str,
}

impl ProofLanguage {
    const CAMEL_NAMES: [&'static str; 4] = [
        "testStartsWithANewContext",
        "testStartsWithANewContextAgain",
        "testTakesEachInput[1]",
        "testTakesEachInput[2]",
    ];
    const SNAKE_NAMES: [&'static str; 4] = [
        "test_starts_with_a_new_context",
        "test_starts_with_a_new_context_again",
        "test_takes_each_input[1]",
        "test_takes_each_input[2]",
    ];
    const GO_NAMES: [&'static str; 4] = [
        "TestStartsWithANewContext",
        "TestStartsWithANewContextAgain",
        "TestTakesEachInput[1]",
        "TestTakesEachInput[2]",
    ];
    const LANGUAGES: [Self; 16] = [
        Self {
            extension: "js",
            program: "node",
            flag: "--version",
            names: Self::CAMEL_NAMES,
            yon: true,
            tac: true,
            profiles: r#"@UseProfiles("test")
"#,
            settings: r#"@TestSettings({"yon.proof.mode":"on"})
"#,
            setup: r#"@TestSetup
@WhenProfile("test")
@WhenSetting({"name":"YON_PROOF_MODE","havingValue":"on","matchIfMissing":false})
class ProofTestSetup {
  static configure() { return {}; }
}
"#,
            unit: r#"@Test(ProofTestSetup)
@UseProfiles("test")
@TestSettings({"yon.proof.mode":"on"})
@FreshStart
export default class ProofServiceTest {
  static contexts = [];

  async testStartsWithANewContext() {
    Assert.isTrue(!ProofServiceTest.contexts.includes(this.behaviors), 'a new context');
    ProofServiceTest.contexts.push(this.behaviors);
  }

  async testStartsWithANewContextAgain() {
    Assert.isTrue(!ProofServiceTest.contexts.includes(this.behaviors), 'a new context');
    ProofServiceTest.contexts.push(this.behaviors);
  }

  @ValueSource([1, 2])
  async testTakesEachInput(value) {
    Assert.isTrue(value > 0, 'a positive input');
  }
}
"#,
        },
        Self {
            extension: "ts",
            program: "bun",
            flag: "--version",
            names: Self::CAMEL_NAMES,
            yon: true,
            tac: true,
            profiles: r#"@UseProfiles("test")
"#,
            settings: r#"@TestSettings({"yon.proof.mode":"on"})
"#,
            setup: r#"@TestSetup
@WhenProfile("test")
@WhenSetting({"name":"YON_PROOF_MODE","havingValue":"on","matchIfMissing":false})
class ProofTestSetup {
  static configure() { return {}; }
}
"#,
            unit: r#"@Test(ProofTestSetup)
@UseProfiles("test")
@TestSettings({"yon.proof.mode":"on"})
@FreshStart
export default class ProofServiceTest {
  static contexts: unknown[] = [];

  async testStartsWithANewContext() {
    Assert.isTrue(!ProofServiceTest.contexts.includes(this.behaviors), 'a new context');
    ProofServiceTest.contexts.push(this.behaviors);
  }

  async testStartsWithANewContextAgain() {
    Assert.isTrue(!ProofServiceTest.contexts.includes(this.behaviors), 'a new context');
    ProofServiceTest.contexts.push(this.behaviors);
  }

  @ValueSource([1, 2])
  async testTakesEachInput(value: number) {
    Assert.isTrue(value > 0, 'a positive input');
  }
}
"#,
        },
        Self {
            extension: "py",
            program: "python3",
            flag: "--version",
            names: Self::SNAKE_NAMES,
            yon: true,
            tac: true,
            profiles: r#"@UseProfiles("test")
"#,
            settings: r#"@TestSettings({"yon.proof.mode":"on"})
"#,
            setup: r#"@TestSetup
@when_profile("test")
@when_setting({"name":"YON_PROOF_MODE","havingValue":"on","matchIfMissing":false})
class ProofTestSetup:
    @staticmethod
    def configure(execution):
        return {}
"#,
            unit: r#"@Test(ProofTestSetup)
@UseProfiles("test")
@TestSettings({"yon.proof.mode":"on"})
@FreshStart
class ProofServiceTest:
    contexts = []

    async def test_starts_with_a_new_context(self):
        Assert.is_true(all(seen is not self.behaviors for seen in ProofServiceTest.contexts), 'a new context')
        ProofServiceTest.contexts.append(self.behaviors)

    async def test_starts_with_a_new_context_again(self):
        Assert.is_true(all(seen is not self.behaviors for seen in ProofServiceTest.contexts), 'a new context')
        ProofServiceTest.contexts.append(self.behaviors)

    @ValueSource([1, 2])
    async def test_takes_each_input(self, value):
        Assert.is_true(value > 0, 'a positive input')
"#,
        },
        Self {
            extension: "php",
            program: "php-cgi",
            flag: "-v",
            names: Self::CAMEL_NAMES,
            yon: true,
            tac: false,
            profiles: r#"#[UseProfiles("test")]
"#,
            settings: r#"#[TestSettings({"yon.proof.mode":"on"})]
"#,
            setup: r#"<?php
#[TestSetup]
#[WhenProfile("test")]
#[WhenSetting({"name":"YON_PROOF_MODE","havingValue":"on","matchIfMissing":false})]
class ProofTestSetup {
    public static function configure($execution) { return []; }
}
"#,
            unit: r#"<?php
#[Test(ProofTestSetup::class)]
#[UseProfiles("test")]
#[TestSettings({"yon.proof.mode":"on"})]
#[FreshStart]
class ProofServiceTest {
    public static array $contexts = [];

    public function testStartsWithANewContext(): void {
        Assert::isTrue(!in_array($this->behaviors, self::$contexts, true), 'a new context');
        self::$contexts[] = $this->behaviors;
    }

    public function testStartsWithANewContextAgain(): void {
        Assert::isTrue(!in_array($this->behaviors, self::$contexts, true), 'a new context');
        self::$contexts[] = $this->behaviors;
    }

    #[ValueSource([1, 2])]
    public function testTakesEachInput(int $value): void {
        Assert::isTrue($value > 0, 'a positive input');
    }
}
"#,
        },
        Self {
            extension: "java",
            program: "javac",
            flag: "-version",
            names: Self::CAMEL_NAMES,
            yon: true,
            tac: true,
            profiles: r#"@UseProfiles("test")
"#,
            settings: r#"@TestSettings({"yon.proof.mode":"on"})
"#,
            setup: r#"@TestSetup
@WhenProfile("test")
@WhenSetting({"name":"YON_PROOF_MODE","havingValue":"on","matchIfMissing":false})
class ProofTestSetup {
    static Behaviors.Configuration configure(Behaviors.Execution execution) {
        return new Behaviors.Configuration(java.util.Map.of(), java.util.Map.of(), java.util.Map.of(), null, null);
    }
}
"#,
            unit: r#"@Test(ProofTestSetup.class)
@UseProfiles("test")
@TestSettings({"yon.proof.mode":"on"})
@FreshStart
class ProofServiceTest {
    static java.util.List<Object> contexts = new java.util.ArrayList<>();

    void testStartsWithANewContext() {
        Assert.isTrue(contexts.stream().noneMatch(seen -> seen == behaviors), "a new context");
        contexts.add(behaviors);
    }

    void testStartsWithANewContextAgain() {
        Assert.isTrue(contexts.stream().noneMatch(seen -> seen == behaviors), "a new context");
        contexts.add(behaviors);
    }

    @ValueSource([1, 2])
    void testTakesEachInput(int value) {
        Assert.isTrue(value > 0, "a positive input");
    }
}
"#,
        },
        Self {
            extension: "groovy",
            program: "groovyc",
            flag: "--version",
            names: Self::CAMEL_NAMES,
            yon: true,
            tac: false,
            profiles: r#"@UseProfiles("test")
"#,
            settings: r#"@TestSettings({"yon.proof.mode":"on"})
"#,
            setup: r#"@TestSetup
@WhenProfile("test")
@WhenSetting({"name":"YON_PROOF_MODE","havingValue":"on","matchIfMissing":false})
class ProofTestSetup {
    static Behaviors.Configuration configure(Behaviors.Execution execution) {
        return new Behaviors.Configuration([:], [:], [:], null, null)
    }
}
"#,
            unit: r#"@Test(ProofTestSetup.class)
@UseProfiles("test")
@TestSettings({"yon.proof.mode":"on"})
@FreshStart
class ProofServiceTest {
    static List contexts = []

    void testStartsWithANewContext() {
        Assert.isTrue(!contexts.any { it.is(behaviors) }, 'a new context')
        contexts.add(behaviors)
    }

    void testStartsWithANewContextAgain() {
        Assert.isTrue(!contexts.any { it.is(behaviors) }, 'a new context')
        contexts.add(behaviors)
    }

    @ValueSource([1, 2])
    void testTakesEachInput(int value) {
        Assert.isTrue(value > 0, 'a positive input')
    }
}
"#,
        },
        Self {
            extension: "kt",
            program: "kotlinc",
            flag: "-version",
            names: Self::CAMEL_NAMES,
            yon: true,
            tac: true,
            profiles: r#"@UseProfiles("test")
"#,
            settings: r#"@TestSettings({"yon.proof.mode":"on"})
"#,
            setup: r#"@TestSetup
@WhenProfile("test")
@WhenSetting({"name":"YON_PROOF_MODE","havingValue":"on","matchIfMissing":false})
class ProofTestSetup {
    companion object {
        fun configure(execution: Behaviors.Execution): Behaviors.Configuration = Behaviors.Configuration()
    }
}
"#,
            unit: r#"@Test(ProofTestSetup::class)
@UseProfiles("test")
@TestSettings({"yon.proof.mode":"on"})
@FreshStart
class ProofServiceTest {
    companion object {
        val contexts = mutableListOf<Any?>()
    }

    fun testStartsWithANewContext() {
        Assert.isTrue(contexts.none { it === behaviors }, "a new context")
        contexts.add(behaviors)
    }

    fun testStartsWithANewContextAgain() {
        Assert.isTrue(contexts.none { it === behaviors }, "a new context")
        contexts.add(behaviors)
    }

    @ValueSource([1, 2])
    fun testTakesEachInput(value: Int) {
        Assert.isTrue(value > 0, "a positive input")
    }
}
"#,
        },
        Self {
            extension: "cs",
            program: "dotnet",
            flag: "--version",
            names: Self::CAMEL_NAMES,
            yon: true,
            tac: true,
            profiles: r#"[UseProfiles("test")]
"#,
            settings: r#"[TestSettings({"yon.proof.mode":"on"})]
"#,
            setup: r#"[TestSetup]
[WhenProfile("test")]
[WhenSetting({"name":"YON_PROOF_MODE","havingValue":"on","matchIfMissing":false})]
public sealed class ProofTestSetup {
    public static BehaviorAdapters configure(BehaviorExecution execution) { return new BehaviorAdapters(); }
}
"#,
            unit: r#"[Test(typeof(ProofTestSetup))]
[UseProfiles("test")]
[TestSettings({"yon.proof.mode":"on"})]
[FreshStart]
sealed class ProofServiceTest {
    static System.Collections.Generic.List<object> contexts = new System.Collections.Generic.List<object>();

    void testStartsWithANewContext() {
        Assert.isTrue(!contexts.Exists(seen => ReferenceEquals(seen, this.behaviors)), "a new context");
        contexts.Add(this.behaviors);
    }

    void testStartsWithANewContextAgain() {
        Assert.isTrue(!contexts.Exists(seen => ReferenceEquals(seen, this.behaviors)), "a new context");
        contexts.Add(this.behaviors);
    }

    [ValueSource([1, 2])]
    void testTakesEachInput(int value) {
        Assert.isTrue(value > 0, "a positive input");
    }
}
"#,
        },
        Self {
            extension: "fs",
            program: "dotnet",
            flag: "--version",
            names: Self::CAMEL_NAMES,
            yon: true,
            tac: true,
            profiles: r#"[<UseProfiles("test")>]
"#,
            settings: r#"[<TestSettings({"yon.proof.mode":"on"})>]
"#,
            setup: r#"[<TestSetup>]
[<WhenProfile("test")>]
[<WhenSetting({"name":"YON_PROOF_MODE","havingValue":"on","matchIfMissing":false})>]
type ProofTestSetup() =
    static member configure(execution: BehaviorExecution) : BehaviorAdapters =
        BehaviorAdapters()
"#,
            unit: r#"[<Test(typeof<ProofTestSetup>)>]
[<UseProfiles("test")>]
[<TestSettings({"yon.proof.mode":"on"})>]
[<FreshStart>]
type ProofServiceTest() =
    static let contexts = System.Collections.Generic.List<obj>()

    member this.testStartsWithANewContext() : unit =
        Assert.isTrue(not (contexts.Exists(fun seen -> obj.ReferenceEquals(seen, this.behaviors))), "a new context")
        contexts.Add(box this.behaviors)

    member this.testStartsWithANewContextAgain() : unit =
        Assert.isTrue(not (contexts.Exists(fun seen -> obj.ReferenceEquals(seen, this.behaviors))), "a new context")
        contexts.Add(box this.behaviors)

    [<ValueSource([1, 2])>]
    member this.testTakesEachInput(value: int) : unit =
        Assert.isTrue(value > 0, "a positive input")
"#,
        },
        Self {
            extension: "dart",
            program: "dart",
            flag: "--version",
            names: Self::CAMEL_NAMES,
            yon: true,
            tac: true,
            profiles: r#"@UseProfiles("test")
"#,
            settings: r#"@TestSettings({"yon.proof.mode":"on"})
"#,
            setup: r#"@TestSetup
@WhenProfile("test")
@WhenSetting({"name":"YON_PROOF_MODE","havingValue":"on","matchIfMissing":false})
class ProofTestSetup {
  static BehaviorAdapters configure(BehaviorExecution execution) {
    return BehaviorAdapters();
  }
}
"#,
            unit: r#"@Test(ProofTestSetup)
@UseProfiles("test")
@TestSettings({"yon.proof.mode":"on"})
@FreshStart
class ProofServiceTest {
  static final List<Object?> contexts = [];

  Future<void> testStartsWithANewContext() async {
    Assert.isTrue(!contexts.any((seen) => identical(seen, behaviors)), 'a new context');
    contexts.add(behaviors);
  }

  Future<void> testStartsWithANewContextAgain() async {
    Assert.isTrue(!contexts.any((seen) => identical(seen, behaviors)), 'a new context');
    contexts.add(behaviors);
  }

  @ValueSource([1, 2])
  Future<void> testTakesEachInput(int value) async {
    Assert.isTrue(value > 0, 'a positive input');
  }
}
"#,
        },
        Self {
            extension: "rs",
            program: "rustc",
            flag: "--version",
            names: Self::CAMEL_NAMES,
            yon: true,
            tac: true,
            profiles: r#"#[UseProfiles("test")]
"#,
            settings: r#"#[TestSettings({"yon.proof.mode":"on"})]
"#,
            setup: r#"#[TestSetup]
#[WhenProfile("test")]
#[WhenSetting({"name":"YON_PROOF_MODE","havingValue":"on","matchIfMissing":false})]
struct ProofTestSetup {}
impl ProofTestSetup {
    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> { Ok(BehaviorConfiguration::default()) }
}
"#,
            unit: r#"#[Test(ProofTestSetup)]
#[UseProfiles("test")]
#[TestSettings({"yon.proof.mode":"on"})]
#[FreshStart]
struct ProofServiceTest {
    behaviors: std::cell::RefCell<Option<BehaviorContext>>,
    contexts: std::cell::RefCell<Vec<BehaviorContext>>,
}
impl ProofServiceTest {
    fn new() -> Self { Self { behaviors: std::cell::RefCell::new(None), contexts: std::cell::RefCell::new(Vec::new()) } }
    fn keep_a_new_context(&self) {
        let current = self.behaviors.borrow().clone().expect("the case has a context");
        let seen = self.contexts.borrow().iter().any(|context| std::sync::Arc::ptr_eq(&context.0, &current.0));
        Assert::is_true_with_message(!seen, "a new context");
        self.contexts.borrow_mut().push(current);
    }
    fn testStartsWithANewContext(&self) {
        self.keep_a_new_context();
    }
    fn testStartsWithANewContextAgain(&self) {
        self.keep_a_new_context();
    }
    #[ValueSource([1, 2])]
    fn testTakesEachInput(&self, value: i64) {
        Assert::is_true_with_message(value > 0, "a positive input");
    }
}
"#,
        },
        Self {
            extension: "go",
            program: "go",
            flag: "version",
            names: Self::GO_NAMES,
            yon: true,
            tac: true,
            profiles: r#"// @UseProfiles("test")
"#,
            settings: r#"// @TestSettings({"yon.proof.mode":"on"})
"#,
            setup: r#"// @TestSetup
// @WhenProfile("test")
// @WhenSetting({"name":"YON_PROOF_MODE","havingValue":"on","matchIfMissing":false})
type ProofTestSetup struct{}

func (ProofTestSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {
	return NewBehaviorConfiguration(), nil
}
"#,
            unit: r#"// @Test(ProofTestSetup)
// @UseProfiles("test")
// @TestSettings({"yon.proof.mode":"on"})
// @FreshStart
type ProofServiceTest struct {
	behaviors *BehaviorContext
	contexts  []*BehaviorContext
}

func (test *ProofServiceTest) keepANewContext() {
	for _, seen := range test.contexts {
		Assert.IsTrue(seen != test.behaviors, "a new context")
	}
	test.contexts = append(test.contexts, test.behaviors)
}

func (test *ProofServiceTest) TestStartsWithANewContext() {
	test.keepANewContext()
}

func (test *ProofServiceTest) TestStartsWithANewContextAgain() {
	test.keepANewContext()
}

// @ValueSource([1, 2])
func (test *ProofServiceTest) TestTakesEachInput(value int) {
	Assert.IsTrue(value > 0, "a positive input")
}
"#,
        },
        Self {
            extension: "rb",
            program: "ruby",
            flag: "--version",
            names: Self::SNAKE_NAMES,
            yon: true,
            tac: true,
            profiles: r#"#[UseProfiles("test")]
"#,
            settings: r#"#[TestSettings({"yon.proof.mode":"on"})]
"#,
            setup: r#"#[TestSetup]
#[WhenProfile("test")]
#[WhenSetting({"name":"YON_PROOF_MODE","havingValue":"on","matchIfMissing":false})]
class ProofTestSetup
  def self.configure(execution)
    {}
  end
end
"#,
            unit: r#"#[Test(ProofTestSetup)]
#[UseProfiles("test")]
#[TestSettings({"yon.proof.mode":"on"})]
#[FreshStart]
class ProofServiceTest
  @@contexts = []

  def test_starts_with_a_new_context
    Assert.is_true(@@contexts.none? { |seen| seen.equal?(@behaviors) }, "a new context")
    @@contexts << @behaviors
  end

  def test_starts_with_a_new_context_again
    Assert.is_true(@@contexts.none? { |seen| seen.equal?(@behaviors) }, "a new context")
    @@contexts << @behaviors
  end

  #[ValueSource([1, 2])]
  def test_takes_each_input(value)
    Assert.is_true(value > 0, "a positive input")
  end
end
"#,
        },
        Self {
            extension: "lua",
            program: "lua",
            flag: "-v",
            names: Self::SNAKE_NAMES,
            yon: true,
            tac: true,
            profiles: r#"-- @UseProfiles("test")
"#,
            settings: r#"-- @TestSettings({"yon.proof.mode":"on"})
"#,
            setup: r#"-- @TestSetup
-- @WhenProfile("test")
-- @WhenSetting({"name":"YON_PROOF_MODE","havingValue":"on","matchIfMissing":false})
ProofTestSetup = {}

function ProofTestSetup.configure(execution)
  return {}
end
"#,
            unit: r#"-- @Test(ProofTestSetup)
-- @UseProfiles("test")
-- @TestSettings({"yon.proof.mode":"on"})
-- @FreshStart
ProofServiceTest = {
  contexts = {},
}

function ProofServiceTest:keep_a_new_context()
  for _, seen in ipairs(ProofServiceTest.contexts) do
    Assert.is_true(seen ~= self.behaviors, "a new context")
  end
  table.insert(ProofServiceTest.contexts, self.behaviors)
end

function ProofServiceTest:test_starts_with_a_new_context()
  self:keep_a_new_context()
end

function ProofServiceTest:test_starts_with_a_new_context_again()
  self:keep_a_new_context()
end

-- @ValueSource([1, 2])
function ProofServiceTest:test_takes_each_input(value)
  Assert.is_true(value > 0, "a positive input")
end
"#,
        },
        Self {
            extension: "swift",
            program: "swiftc",
            flag: "--version",
            names: Self::CAMEL_NAMES,
            yon: false,
            tac: true,
            profiles: r#"@UseProfiles("test")
"#,
            settings: r#"@TestSettings({"yon.proof.mode":"on"})
"#,
            setup: r#"@TestSetup
@WhenProfile("test")
@WhenSetting({"name":"YON_PROOF_MODE","havingValue":"on","matchIfMissing":false})
final class ProofTestSetup {
    static func configure(_ execution: BehaviorExecution) -> BehaviorAdapters {
        BehaviorAdapters()
    }
}
"#,
            unit: r#"@Test(ProofTestSetup)
@UseProfiles("test")
@TestSettings({"yon.proof.mode":"on"})
@FreshStart
final class ProofServiceTest {
    static var contexts: [AnyObject] = []

    func keepANewContext() throws {
        let current = behaviors as AnyObject
        try Assert.isTrue(!ProofServiceTest.contexts.contains { $0 === current }, "a new context")
        ProofServiceTest.contexts.append(current)
    }

    func testStartsWithANewContext() async throws {
        try keepANewContext()
    }

    func testStartsWithANewContextAgain() async throws {
        try keepANewContext()
    }

    @ValueSource([1, 2])
    func testTakesEachInput(_ value: Int) async throws {
        try Assert.isTrue(value > 0, "a positive input")
    }
}
"#,
        },
        Self {
            extension: "hpp",
            program: "clang++",
            flag: "--version",
            names: Self::CAMEL_NAMES,
            yon: true,
            tac: true,
            profiles: r#", UseProfiles("test")"#,
            settings: r#", TestSettings({"yon.proof.mode":"on"})"#,
            setup: r#"#pragma once

class [[TestSetup, WhenProfile("test"), WhenSetting({"name":"YON_PROOF_MODE","havingValue":"on","matchIfMissing":false})]] ProofTestSetup {
public:
    static BehaviorAdapters configure(const BehaviorExecution&) {
        return BehaviorAdapters();
    }
};
"#,
            unit: r#"#pragma once

class [[Test(ProofTestSetup), UseProfiles("test"), TestSettings({"yon.proof.mode":"on"}), FreshStart]] ProofServiceTest {
public:
    BehaviorContext behaviors;

    void testStartsWithANewContext() {
        Assert::equals(std::vector<std::string>{"test"}, behaviors.profiles());
    }

    void testStartsWithANewContextAgain() {
        Assert::equals(std::vector<std::string>{"test"}, behaviors.profiles());
    }

    [[ValueSource([1, 2])]]
    void testTakesEachInput(int value) {
        Assert::isTrue(value > 0);
    }
};
"#,
        },
    ];

    fn available(&self) -> bool {
        let available = std::process::Command::new(self.program)
            .arg(self.flag)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|status| status.success());
        if !available {
            assert_ne!(
                std::env::var("TAC_TEST_UNITS_REQUIRE_TOOLCHAINS").as_deref(),
                Ok("1"),
                "Required test unit toolchain {} is unavailable",
                self.program
            );
            eprintln!(
                "skipping {}: {} is unavailable",
                self.extension, self.program
            );
        }
        available
    }

    fn in_realm(source: &str, realm: &str) -> String {
        if realm == "yon" {
            return source.to_owned();
        }
        source
            .replace("YON_PROOF_MODE", "TAC_PROOF_MODE")
            .replace("\"yon.proof.mode\"", "\"tac.proof.mode\"")
    }

    fn write(root: &std::path::Path, relative: &str, source: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directories");
        std::fs::write(path, source).expect("source");
    }

    fn project(&self, realm: &str) -> tempfile::TempDir {
        let folder = MarkerProof::folder(realm);
        let root = tempfile::tempdir().expect("project");
        let extension = self.extension;
        let setup = Self::in_realm(self.setup, realm);
        Self::write(
            root.path(),
            &format!("{folder}/tests/ProofTestSetup.{extension}"),
            &setup,
        );
        for (name, unit) in self.units(realm) {
            Self::write(
                root.path(),
                &format!("{folder}/tests/{name}.{extension}"),
                &unit,
            );
        }
        root
    }

    fn units(&self, realm: &str) -> [(&'static str, String); 3] {
        let unit = Self::in_realm(self.unit, realm);
        let profiles = Self::in_realm(self.profiles, realm);
        let settings = Self::in_realm(self.settings, realm);
        let without_profiles = unit
            .replace(&profiles, "")
            .replace("ProofServiceTest", "NoProfileTest");
        let without_settings = unit
            .replace(&settings, "")
            .replace("ProofServiceTest", "NoSettingTest");
        [
            ("ProofServiceTest", unit),
            ("NoProfileTest", without_profiles),
            ("NoSettingTest", without_settings),
        ]
    }

    fn check(&self, realm: &str, name: &str, outcome: Result<Vec<MarkerCase>, String>) {
        let label = format!("{realm} {} {name}", self.extension);
        if name != "ProofServiceTest" {
            MarkerProof::check_inactive(&label, outcome);
            return;
        }
        let cases = outcome.unwrap_or_else(|error| panic!("{label}: {error}"));
        let names: Vec<&str> = cases.iter().map(|case| case.name.as_str()).collect();
        assert_eq!(names, self.names, "{label}: {cases:?}");
        for case in &cases {
            assert!(case.passed, "{label} {}: {}", case.name, case.error);
        }
    }
}

#[derive(Debug)]
struct MarkerCase {
    name: String,
    passed: bool,
    error: String,
}

struct MarkerProof;

impl MarkerProof {
    const INACTIVE: [&'static str; 3] = ["inactive", "Inactive", "exited unsuccessfully"];
    const ASYNC_SETUP: &'static str = "[TestSetup]\npublic sealed class AsyncTestSetup {\n    public static BehaviorAdapters configure(BehaviorExecution execution) { return new BehaviorAdapters(); }\n}\n";
    const ASYNC_UNIT: &'static str = "[Test(typeof(AsyncTestSetup))]\npublic sealed class AsyncServiceTest {\n    public async System.Threading.Tasks.Task testFailsAfterAnAwait() {\n        await System.Threading.Tasks.Task.Yield();\n        Assert.equals(1, 2, \"after an await\");\n    }\n\n    public async System.Threading.Tasks.Task testPassesAfterAnAwait() {\n        await System.Threading.Tasks.Task.Yield();\n        Assert.equals(2, 2);\n    }\n}\n";

    fn folder(realm: &str) -> &'static str {
        if realm == "yon" {
            return "server";
        }
        "client"
    }

    fn check_inactive(label: &str, outcome: Result<Vec<MarkerCase>, String>) {
        let errors = match outcome {
            Err(error) => vec![error],
            Ok(cases) => {
                assert!(!cases.is_empty(), "{label}: no case ran");
                assert!(
                    cases.iter().all(|case| !case.passed),
                    "{label}: an inactive TestSetup let a case pass: {cases:?}"
                );
                cases.into_iter().map(|case| case.error).collect()
            }
        };
        for error in errors {
            assert!(
                Self::INACTIVE.iter().any(|text| error.contains(text)),
                "{label}: {error}"
            );
        }
    }

    async fn yon_cases(
        root: &std::path::Path,
        relative: &str,
        extension: &str,
    ) -> Result<Vec<MarkerCase>, String> {
        let source =
            HandlerSource::discover(root, relative).map_err(|error| format!("{error:?}"))?;
        let mut options = HandlerSupervisorOptions::default();
        if extension == "ts" {
            options.runtimes.javascript = "bun".into();
        }
        let supervisor = HandlerSupervisor::new(options).expect("supervisor");
        let response = supervisor
            .invoke(
                &source,
                &HandlerRequest::route("test_unit", "/", HttpMethod::Post),
                &HandlerCancellation::default(),
            )
            .await
            .map_err(|error| format!("{error:?}"))?;
        if let Some(error) = response.error {
            return Err(format!("{error:?}"));
        }
        let body = response.body.expect("body").data;
        let report: serde_json::Value = serde_json::from_str(&body).expect("JSON report");
        Ok(report["cases"]
            .as_array()
            .expect("cases")
            .iter()
            .map(|case| MarkerCase {
                name: case["name"].as_str().unwrap_or_default().to_owned(),
                passed: case["passed"] == true,
                error: case["error"].as_str().unwrap_or_default().to_owned(),
            })
            .collect())
    }

    fn tac_cases(root: &std::path::Path, relative: &str) -> Result<Vec<MarkerCase>, String> {
        let report = tachyon_core::client_tests::run(root, std::path::Path::new(relative))
            .map_err(|error| format!("{error:?}"))?;
        Ok(report
            .cases
            .into_iter()
            .map(|case| MarkerCase {
                name: case.name,
                passed: case.passed,
                error: case.error.unwrap_or_default(),
            })
            .collect())
    }
}

#[tokio::test]
async fn yon_test_settings_profiles_fresh_start_and_inputs_run_in_every_language() {
    for language in ProofLanguage::LANGUAGES {
        if !language.yon || !language.available() {
            continue;
        }
        let root = language.project("yon");
        for (name, _) in language.units("yon") {
            let relative = format!("server/tests/{name}.{}", language.extension);
            let outcome = MarkerProof::yon_cases(root.path(), &relative, language.extension).await;
            language.check("yon", name, outcome);
        }
    }
}

#[test]
fn tac_test_settings_profiles_fresh_start_and_inputs_run_in_every_language() {
    for language in ProofLanguage::LANGUAGES {
        if !language.tac || !language.available() {
            continue;
        }
        let root = language.project("tac");
        for (name, _) in language.units("tac") {
            let relative = format!("client/tests/{name}.{}", language.extension);
            let outcome = MarkerProof::tac_cases(root.path(), &relative);
            language.check("tac", name, outcome);
        }
    }
}

struct DescribedService {
    extension: &'static str,
    program: &'static str,
    flag: &'static str,
    unit: &'static str,
    files: [(&'static str, &'static str); 3],
}

impl DescribedService {
    const SERVICES: [Self; 11] = [
        Self {
            extension: "js",
            program: "node",
            flag: "--version",
            unit: "client/tests/PriceServiceTest.js",
            files: [
                (
                    "client/services/PriceService.js",
                    r#"@Description("Prices one item and keeps the price")
@Service
@CacheSettings({"name":"prices","ttlMs":30000,"maxBytes":65536,"maxEntries":64})
export default class PriceService {
  behaviors;
  reads = 0;

  @Cached
  async price(@CacheKey sku, note) {
    this.reads += 1;
    return { sku, note, reads: this.reads };
  }
}
"#,
                ),
                (
                    "client/tests/PriceServiceTest.js",
                    r"@Test(PriceTestSetup)
export default class PriceServiceTest {
  async testCacheKeyLeavesTheOtherParameterOut() {
    const prices = this.construct(PriceService);
    prices.behaviors = this.behaviors;
    const first = await prices.price('anvil', 'first');
    const sameKey = await prices.price('anvil', 'second');
    const otherKey = await prices.price('lamp', 'third');
    Assert.equals({ sku: 'anvil', note: 'first', reads: 1 }, first);
    Assert.equals(first, sameKey, 'a parameter without CacheKey is not part of the key');
    Assert.equals({ sku: 'lamp', note: 'third', reads: 2 }, otherKey, 'a different CacheKey value reads again');
  }
}
",
                ),
                (
                    "client/tests/PriceTestSetup.js",
                    r"@TestSetup
class PriceTestSetup {
  static configure() {
    const entries = new Map();
    return {
      authorize: () => true,
      caches: {
        prices: {
          get: (key) => entries.get(key) ?? { hit: false },
          set: (key, value, policy) => { entries.set(key, { hit: true, value, expiresAt: policy.expiresAt }); },
          delete: (key) => { entries.delete(key); },
        },
      },
    };
  }
}
",
                ),
            ],
        },
        Self {
            extension: "ts",
            program: "bun",
            flag: "--version",
            unit: "client/tests/PriceServiceTest.ts",
            files: [
                (
                    "client/services/PriceService.ts",
                    r#"@Description("Prices one item and keeps the price")
@Service
@CacheSettings({"name":"prices","ttlMs":30000,"maxBytes":65536,"maxEntries":64})
export default class PriceService {
  behaviors: unknown;
  reads = 0;

  @Cached
  async price(@CacheKey sku: string, note: string) {
    this.reads += 1;
    return { sku, note, reads: this.reads };
  }
}
"#,
                ),
                (
                    "client/tests/PriceServiceTest.ts",
                    r"@Test(PriceTestSetup)
export default class PriceServiceTest {
  async testCacheKeyLeavesTheOtherParameterOut() {
    const prices = this.construct(PriceService);
    prices.behaviors = this.behaviors;
    const first = await prices.price('anvil', 'first');
    const sameKey = await prices.price('anvil', 'second');
    const otherKey = await prices.price('lamp', 'third');
    Assert.equals({ sku: 'anvil', note: 'first', reads: 1 }, first);
    Assert.equals(first, sameKey, 'a parameter without CacheKey is not part of the key');
    Assert.equals({ sku: 'lamp', note: 'third', reads: 2 }, otherKey, 'a different CacheKey value reads again');
  }
}
",
                ),
                (
                    "client/tests/PriceTestSetup.ts",
                    r"@TestSetup
class PriceTestSetup {
  static configure() {
    const entries = new Map<string, unknown>();
    return {
      authorize: () => true,
      caches: {
        prices: {
          get: (key: string) => entries.get(key) ?? { hit: false },
          set: (key: string, value: unknown, policy: { expiresAt: number }) => { entries.set(key, { hit: true, value, expiresAt: policy.expiresAt }); },
          delete: (key: string) => { entries.delete(key); },
        },
      },
    };
  }
}
",
                ),
            ],
        },
        Self {
            extension: "dart",
            program: "dart",
            flag: "--version",
            unit: "client/tests/PriceServiceTest.dart",
            files: [
                (
                    "client/services/PriceService.dart",
                    r#"@Description("Prices one item and keeps the price")
@Service
@CacheSettings({"name":"prices","ttlMs":30000,"maxBytes":65536,"maxEntries":64})
class PriceService {
  @Needs
  final BehaviorContext behaviors;
  int reads = 0;

  @Cached
  Future<Object?> price(@CacheKey String sku, String note) async {
    reads += 1;
    return {'sku': sku, 'note': note, 'reads': reads};
  }
}
"#,
                ),
                (
                    "client/tests/PriceServiceTest.dart",
                    r"@Test(PriceTestSetup)
class PriceServiceTest {
  Future<void> testCacheKeyLeavesTheOtherParameterOut() async {
    final prices = PriceService(behaviors!);
    final first = await prices.price('anvil', 'first');
    final sameKey = await prices.price('anvil', 'second');
    final otherKey = await prices.price('lamp', 'third');
    Assert.equals({'sku': 'anvil', 'note': 'first', 'reads': 1}, first);
    Assert.equals(first, sameKey, 'a parameter without CacheKey is not part of the key');
    Assert.equals({'sku': 'lamp', 'note': 'third', 'reads': 2}, otherKey, 'a different CacheKey value reads again');
  }
}
",
                ),
                (
                    "client/tests/PriceTestSetup.dart",
                    r"@TestSetup
class PriceTestSetup {
  static BehaviorAdapters configure(BehaviorExecution execution) {
    final entries = <String, Map<String, Object?>>{};
    return BehaviorAdapters(
      caches: {
        'prices': BehaviorCacheAdapter(
          get: (key, execution) async => entries[key] ?? <String, Object?>{'hit': false},
          set: (key, value, policy, expiresAt, execution) async {
            entries[key] = {'hit': true, 'value': value, 'expiresAt': expiresAt};
          },
          delete: (key, execution) async {
            entries.remove(key);
          },
        ),
      },
      authorize: (call, execution) async => true,
    );
  }
}
",
                ),
            ],
        },
        Self {
            extension: "rs",
            program: "rustc",
            flag: "--version",
            unit: "client/tests/PriceServiceTest.rs",
            files: [
                (
                    "client/services/PriceService.rs",
                    r#"#[Description("Prices one item and keeps the price")]
#[Service]
#[CacheSettings({"name":"prices","ttlMs":30000,"maxBytes":65536,"maxEntries":64})]
pub struct PriceService {
    #[Needs]
    behaviors: BehaviorContext,
    reads: std::cell::Cell<f64>,
}
impl PriceService {
    #[Cached]
    pub fn price(&self, #[CacheKey] sku: BehaviorValue, note: BehaviorValue) -> BehaviorResult<BehaviorValue> {
        let reads = self.reads.get() + 1.0;
        self.reads.set(reads);
        Ok(BehaviorValue::object([("sku", sku), ("note", note), ("reads", BehaviorValue::Number(reads))]))
    }
}
"#,
                ),
                (
                    "client/tests/PriceServiceTest.rs",
                    r#"#[Test(PriceTestSetup)]
struct PriceServiceTest {
    #[Fake]
    behaviors: std::cell::RefCell<Option<BehaviorContext>>,
}
impl PriceServiceTest {
    fn new() -> Self {
        Self { behaviors: std::cell::RefCell::new(None) }
    }

    fn testCacheKeyLeavesTheOtherParameterOut(&self) {
        let prices: PriceService = self.construct();
        let first = prices.price(BehaviorValue::from("anvil"), BehaviorValue::from("first")).expect("first price");
        let same_key = prices.price(BehaviorValue::from("anvil"), BehaviorValue::from("second")).expect("same key");
        let other_key = prices.price(BehaviorValue::from("lamp"), BehaviorValue::from("third")).expect("other key");
        Assert::equals(BehaviorValue::object([("sku", BehaviorValue::from("anvil")), ("note", BehaviorValue::from("first")), ("reads", BehaviorValue::Number(1.0))]), first.clone());
        Assert::equals(first, same_key);
        Assert::equals(BehaviorValue::object([("sku", BehaviorValue::from("lamp")), ("note", BehaviorValue::from("third")), ("reads", BehaviorValue::Number(2.0))]), other_key);
    }
}
"#,
                ),
                (
                    "client/tests/PriceTestSetup.rs",
                    r#"#[TestSetup]
struct PriceTestSetup {}
impl PriceTestSetup {
    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {
        #[derive(Default)]
        struct MemoryCache {
            entries: std::sync::Mutex<std::collections::BTreeMap<String, BehaviorCacheEntry>>,
        }
        impl BehaviorCacheAdapter for MemoryCache {
            fn get(&self, key: &str, _execution: &BehaviorExecution) -> BehaviorResult<Option<BehaviorCacheEntry>> {
                Ok(self.entries.lock().expect("read the cache").get(key).cloned())
            }

            fn set(&self, key: &str, value: BehaviorValue, write: &BehaviorCacheWrite) -> BehaviorResult<()> {
                let entry = BehaviorCacheEntry { value, expires_at: write.expires_at };
                self.entries.lock().expect("write the cache").insert(key.to_owned(), entry);
                Ok(())
            }

            fn delete(&self, key: &str, _execution: &BehaviorExecution) -> BehaviorResult<()> {
                self.entries.lock().expect("clear the cache").remove(key);
                Ok(())
            }
        }

        let mut configuration = BehaviorConfiguration::default();
        configuration.caches.insert("prices".into(), std::sync::Arc::new(MemoryCache::default()));
        configuration.authorize = Some(std::sync::Arc::new(|_call, _execution| Ok(true)));
        Ok(configuration)
    }
}
"#,
                ),
            ],
        },
        Self {
            extension: "cs",
            program: "dotnet",
            flag: "--version",
            unit: "client/tests/PriceServiceTest.cs",
            files: [
                (
                    "client/services/PriceService.cs",
                    r#"[Description("Prices one item and keeps the price")]
[Service]
[CacheSettings({"name":"prices","ttlMs":30000,"maxBytes":65536,"maxEntries":64})]
public sealed class PriceService {
    public BehaviorContext behaviors;
    public int reads = 0;

    [Cached]
    public async System.Threading.Tasks.Task<BehaviorJSON> price([CacheKey] BehaviorJSON sku, BehaviorJSON note) {
        await System.Threading.Tasks.Task.CompletedTask;
        reads++;
        return BehaviorJSON.Object(new System.Collections.Generic.Dictionary<string, BehaviorJSON> {
            ["sku"] = sku,
            ["note"] = note,
            ["reads"] = BehaviorJSON.From(reads),
        });
    }
}
"#,
                ),
                (
                    "client/tests/PriceServiceTest.cs",
                    r#"[Test(typeof(PriceTestSetup))]
public sealed class PriceServiceTest {
    public void testCacheKeyLeavesTheOtherParameterOut() {
        var prices = new PriceService();
        prices.behaviors = this.behaviors;
        var first = prices.price(BehaviorJSON.From("anvil"), BehaviorJSON.From("first")).Result;
        var sameKey = prices.price(BehaviorJSON.From("anvil"), BehaviorJSON.From("second")).Result;
        var otherKey = prices.price(BehaviorJSON.From("lamp"), BehaviorJSON.From("third")).Result;
        Assert.equals("{\"note\":\"first\",\"reads\":1,\"sku\":\"anvil\"}", first.ToString());
        Assert.equals(first.ToString(), sameKey.ToString(), "a parameter without CacheKey is not part of the key");
        Assert.equals("{\"note\":\"third\",\"reads\":2,\"sku\":\"lamp\"}", otherKey.ToString(), "a different CacheKey value reads again");
    }
}
"#,
                ),
                (
                    "client/tests/PriceTestSetup.cs",
                    r#"[TestSetup]
public sealed class PriceTestSetup {
    public static BehaviorAdapters configure(BehaviorExecution execution) {
        var adapters = new BehaviorAdapters();
        adapters.caches["prices"] = new MemoryPrices();
        adapters.authorize = (call, current) => System.Threading.Tasks.Task.FromResult(true);
        return adapters;
    }

    public sealed class MemoryPrices : BehaviorCacheAdapter {
        private readonly System.Collections.Generic.Dictionary<string, BehaviorJSON> entries = new System.Collections.Generic.Dictionary<string, BehaviorJSON>();

        public System.Threading.Tasks.Task<BehaviorJSON> get(string key, BehaviorExecution execution) {
            if (entries.TryGetValue(key, out var entry)) return System.Threading.Tasks.Task.FromResult(entry);
            return System.Threading.Tasks.Task.FromResult(BehaviorJSON.Parse("{\"hit\":false}"));
        }

        public System.Threading.Tasks.Task set(string key, BehaviorJSON value, BehaviorJSON policy, long expiresAt, BehaviorExecution execution) {
            entries[key] = BehaviorJSON.Object(new System.Collections.Generic.Dictionary<string, BehaviorJSON> {
                ["hit"] = BehaviorJSON.From(true),
                ["value"] = value,
                ["expiresAt"] = BehaviorJSON.From(expiresAt),
            });
            return System.Threading.Tasks.Task.CompletedTask;
        }

        public System.Threading.Tasks.Task delete(string key, BehaviorExecution execution) {
            entries.Remove(key);
            return System.Threading.Tasks.Task.CompletedTask;
        }
    }
}
"#,
                ),
            ],
        },
        Self {
            extension: "kt",
            program: "kotlinc",
            flag: "-version",
            unit: "client/tests/PriceServiceTest.kt",
            files: [
                (
                    "client/services/PriceService.kt",
                    r#"@Description("Prices one item and keeps the price")
@Service
@CacheSettings({"name":"prices","ttlMs":30000,"maxBytes":65536,"maxEntries":64})
class PriceService() {
    private lateinit var behaviors: Behaviors.Context
    var reads: Int = 0

    constructor(context: Behaviors.Context) : this() {
        behaviors = context
    }

    @Cached
    fun price(@CacheKey sku: String, note: String): Any? {
        reads++
        return mapOf("sku" to sku, "note" to note, "reads" to reads)
    }
}
"#,
                ),
                (
                    "client/tests/PriceServiceTest.kt",
                    r#"@Test(PriceTestSetup::class)
class PriceServiceTest {
    fun testCacheKeyLeavesTheOtherParameterOut() {
        val prices = PriceService(behaviors!!)
        val first = prices.price("anvil", "first")
        val sameKey = prices.price("anvil", "second")
        val otherKey = prices.price("lamp", "third")
        Assert.equals(mapOf("sku" to "anvil", "note" to "first", "reads" to 1), first)
        Assert.equals(first, sameKey, "a parameter without CacheKey is not part of the key")
        Assert.equals(mapOf("sku" to "lamp", "note" to "third", "reads" to 2), otherKey, "a different CacheKey value reads again")
    }
}
"#,
                ),
                (
                    "client/tests/PriceTestSetup.kt",
                    r#"@TestSetup
class PriceTestSetup {
    companion object {
        fun configure(execution: Behaviors.Execution): Behaviors.Configuration {
            val entries = mutableMapOf<String, Behaviors.Entry>()
            return Behaviors.Configuration(
                caches = mapOf("prices" to object : Behaviors.CacheAdapter {
                    override fun get(key: String, execution: Behaviors.Execution): Behaviors.Entry = entries[key] ?: Behaviors.Entry()
                    override fun set(key: String, value: Any?, execution: Behaviors.Write) {
                        entries[key] = Behaviors.Entry(true, value, execution.expiresAt)
                    }
                    override fun delete(key: String, execution: Behaviors.Execution) {
                        entries.remove(key)
                    }
                }),
                authorize = Behaviors.Authorization { _, _ -> true }
            )
        }
    }
}
"#,
                ),
            ],
        },
        Self {
            extension: "swift",
            program: "swiftc",
            flag: "--version",
            unit: "client/tests/PriceServiceTest.swift",
            files: [
                (
                    "client/services/PriceService.swift",
                    r#"@Description("Prices one item and keeps the price")
@Service
@CacheSettings({"name":"prices","ttlMs":30000,"maxBytes":65536,"maxEntries":64})
final class PriceService {
    @Needs
    let behaviors: BehaviorContext
    var reads: Int = 0

    @Cached
    func price(@CacheKey _ sku: String, _ note: String) async throws -> BehaviorJSON {
        reads += 1
        return .object(["sku": .string(sku), "note": .string(note), "reads": .number(Double(reads))])
    }
}
"#,
                ),
                (
                    "client/tests/PriceServiceTest.swift",
                    r#"@Test(PriceTestSetup)
final class PriceServiceTest {
    func testCacheKeyLeavesTheOtherParameterOut() async throws {
        let prices = PriceService(behaviors: behaviors!)
        let first = try await prices.price("anvil", "first")
        let sameKey = try await prices.price("anvil", "second")
        let otherKey = try await prices.price("lamp", "third")
        try Assert.equals(BehaviorJSON.object(["sku": .string("anvil"), "note": .string("first"), "reads": .number(1)]), first)
        try Assert.equals(first, sameKey, "a parameter without CacheKey is not part of the key")
        try Assert.equals(BehaviorJSON.object(["sku": .string("lamp"), "note": .string("third"), "reads": .number(2)]), otherKey, "a different CacheKey value reads again")
    }
}
"#,
                ),
                (
                    "client/tests/PriceTestSetup.swift",
                    r#"@TestSetup
final class PriceTestSetup {
    final class Entries {
        var values: [String: BehaviorJSON] = [:]
    }

    static func configure(_ execution: BehaviorExecution) -> BehaviorAdapters {
        let entries = Entries()
        var adapters = BehaviorAdapters()
        adapters.caches["prices"] = BehaviorCacheAdapter(
            get: { key, _ in
                entries.values[key] ?? .object(["hit": .bool(false)])
            },
            set: { key, value, _, expiresAt, _ in
                entries.values[key] = .object(["hit": .bool(true), "value": value, "expiresAt": .number(Double(expiresAt))])
            },
            delete: { key, _ in
                entries.values.removeValue(forKey: key)
            })
        adapters.authorize = { _, _ in true }
        return adapters
    }
}
"#,
                ),
            ],
        },
        Self {
            extension: "go",
            program: "go",
            flag: "version",
            unit: "client/tests/PriceServiceTest.go",
            files: [
                (
                    "client/services/PriceService.go",
                    r#"// @Description("Prices one item and keeps the price")
// @Service
// @CacheSettings({"name":"prices","ttlMs":30000,"maxBytes":65536,"maxEntries":64})
type PriceService struct {
	behaviors *BehaviorContext
	Reads     int
}

// @Cached
func (prices *PriceService) Price(
	// @CacheKey
	sku BehaviorValue,
	note BehaviorValue,
) (BehaviorValue, error) {
	prices.Reads++
	return map[string]any{"sku": sku, "note": note, "reads": prices.Reads}, nil
}
"#,
                ),
                (
                    "client/tests/PriceServiceTest.go",
                    r#"// @Test(PriceTestSetup)
type PriceServiceTest struct {
	// @Fake
	behaviors *BehaviorContext
}

func (test *PriceServiceTest) TestCacheKeyLeavesTheOtherParameterOut() {
	prices := &PriceService{}
	test.construct(prices)
	first, err := prices.Price("anvil", "first")
	Assert.Equals(nil, err)
	sameKey, err := prices.Price("anvil", "second")
	Assert.Equals(nil, err)
	otherKey, err := prices.Price("lamp", "third")
	Assert.Equals(nil, err)
	Assert.Equals(map[string]any{"sku": "anvil", "note": "first", "reads": 1.0}, first)
	Assert.Equals(first, sameKey, "a parameter without CacheKey is not part of the key")
	Assert.Equals(map[string]any{"sku": "lamp", "note": "third", "reads": 2.0}, otherKey, "a different CacheKey value reads again")
}
"#,
                ),
                (
                    "client/tests/PriceTestSetup.go",
                    r#"import "sync"

// @TestSetup
type PriceTestSetup struct {
	lock    sync.Mutex
	entries map[string]BehaviorCacheEntry
}

func (*PriceTestSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {
	config := NewBehaviorConfiguration()
	config.Caches["prices"] = &PriceTestSetup{entries: map[string]BehaviorCacheEntry{}}
	config.Authorize = func(call *BehaviorAuthorizationCall, execution *BehaviorExecution) (bool, error) {
		return true, nil
	}
	return config, nil
}

func (memory *PriceTestSetup) Get(key string, execution *BehaviorExecution) (*BehaviorCacheEntry, error) {
	memory.lock.Lock()
	defer memory.lock.Unlock()
	entry, present := memory.entries[key]
	if !present {
		return nil, nil
	}
	return &entry, nil
}

func (memory *PriceTestSetup) Set(key string, value BehaviorValue, write *BehaviorCacheWrite) error {
	memory.lock.Lock()
	defer memory.lock.Unlock()
	memory.entries[key] = BehaviorCacheEntry{Value: value, ExpiresAt: write.ExpiresAt}
	return nil
}

func (memory *PriceTestSetup) Delete(key string, execution *BehaviorExecution) error {
	memory.lock.Lock()
	defer memory.lock.Unlock()
	delete(memory.entries, key)
	return nil
}
"#,
                ),
            ],
        },
        Self {
            extension: "rb",
            program: "ruby",
            flag: "--version",
            unit: "client/tests/PriceServiceTest.rb",
            files: [
                (
                    "client/services/PriceService.rb",
                    r#"#[Description("Prices one item and keeps the price")]
#[Service]
#[CacheSettings({"name":"prices","ttlMs":30000,"maxBytes":65536,"maxEntries":64})]
class PriceService
  # @return [Integer]
  attr_reader :reads

  def initialize
    @reads = 0
  end

  # @param sku [String]
  # @param note [String]
  # @return [Hash]
  #[Cached]
  def price(
    #[CacheKey]
    sku,
    note
  )
    @reads += 1
    { "sku" => sku, "note" => note, "reads" => @reads }
  end
end
"#,
                ),
                (
                    "client/tests/PriceServiceTest.rb",
                    r#"#[Test(PriceTestSetup)]
class PriceServiceTest
  # @return [void]
  def test_cache_key_leaves_the_other_parameter_out
    prices = construct(PriceService)
    first = prices.price("anvil", "first")
    same_key = prices.price("anvil", "second")
    other_key = prices.price("lamp", "third")
    Assert.equals({ "sku" => "anvil", "note" => "first", "reads" => 1 }, first)
    Assert.equals(first, same_key, "a parameter without CacheKey is not part of the key")
    Assert.equals({ "sku" => "lamp", "note" => "third", "reads" => 2 }, other_key, "a different CacheKey value reads again")
  end
end
"#,
                ),
                (
                    "client/tests/PriceTestSetup.rb",
                    r#"#[TestSetup]
class PriceTestSetup
  # @param execution [BehaviorExecution]
  # @return [Hash]
  def self.configure(execution)
    entries = {}
    {
      caches: {
        prices: {
          get: ->(key, _execution) { entries.fetch(key, { "hit" => false }) },
          set: ->(key, value, execution) { entries[key] = { "hit" => true, "value" => value, "expiresAt" => execution.expires_at } },
          delete: ->(key, _execution) { entries.delete(key) }
        }
      },
      authorize: ->(_call, _execution) { true }
    }
  end
end
"#,
                ),
            ],
        },
        Self {
            extension: "py",
            program: "python3",
            flag: "--version",
            unit: "client/tests/LabelServiceTest.py",
            files: [
                (
                    "client/services/LabelService.py",
                    r#"@description("Gives the label of one item")
@service
class LabelService:
    def label(self, sku: str) -> str:
        return 'item ' + sku
"#,
                ),
                (
                    "client/tests/LabelServiceTest.py",
                    r"@Test(LabelTestSetup)
class LabelServiceTest:
    def test_a_described_service_runs(self):
        Assert.equals('item anvil', LabelService().label('anvil'))
",
                ),
                (
                    "client/tests/LabelTestSetup.py",
                    r"@TestSetup
class LabelTestSetup:
    @staticmethod
    def configure(execution):
        return {}
",
                ),
            ],
        },
        Self {
            extension: "hpp",
            program: "clang++",
            flag: "--version",
            unit: "client/tests/LabelServiceTest.hpp",
            files: [
                (
                    "client/services/LabelService.hpp",
                    r#"#pragma once
#include <string>

class [[Service, Description("Gives the label of one item")]] LabelService {
public:
    std::string label(const std::string& sku) const { return "item " + sku; }
};
"#,
                ),
                (
                    "client/tests/LabelServiceTest.hpp",
                    r#"#pragma once
#include <string>

class [[Test(LabelTestSetup)]] LabelServiceTest {
public:
    void testADescribedServiceRuns() {
        LabelService labels = construct<LabelService>();
        Assert::equals(std::string("item anvil"), labels.label("anvil"));
    }
};
"#,
                ),
                (
                    "client/tests/LabelTestSetup.hpp",
                    r"#pragma once

class [[TestSetup]] LabelTestSetup {
public:
    static BehaviorAdapters configure(const BehaviorExecution&) {
        return BehaviorAdapters();
    }
};
",
                ),
            ],
        },
    ];

    fn available(&self) -> bool {
        let language = ProofLanguage {
            extension: self.extension,
            program: self.program,
            flag: self.flag,
            names: ProofLanguage::CAMEL_NAMES,
            yon: false,
            tac: true,
            profiles: "",
            settings: "",
            setup: "",
            unit: "",
        };
        language.available()
    }
}

#[test]
fn tac_cache_keys_and_descriptions_run_in_every_language() {
    for service in DescribedService::SERVICES {
        if !service.available() {
            continue;
        }
        let root = tempfile::tempdir().expect("project");
        for (relative, source) in service.files {
            ProofLanguage::write(root.path(), relative, source);
        }
        let cases = MarkerProof::tac_cases(root.path(), service.unit)
            .unwrap_or_else(|error| panic!("{}: {error}", service.extension));
        assert_eq!(cases.len(), 1, "{}: {cases:?}", service.extension);
        assert!(cases[0].passed, "{}: {cases:?}", service.extension);
    }
}

#[tokio::test]
async fn a_csharp_case_that_fails_after_an_await_is_reported_as_failed() {
    let language = &ProofLanguage::LANGUAGES[7];
    if !language.available() {
        return;
    }
    for realm in ["yon", "tac"] {
        let folder = MarkerProof::folder(realm);
        let root = tempfile::tempdir().expect("project");
        ProofLanguage::write(
            root.path(),
            &format!("{folder}/tests/AsyncTestSetup.cs"),
            MarkerProof::ASYNC_SETUP,
        );
        let relative = format!("{folder}/tests/AsyncServiceTest.cs");
        ProofLanguage::write(root.path(), &relative, MarkerProof::ASYNC_UNIT);
        let outcome = if realm == "yon" {
            MarkerProof::yon_cases(root.path(), &relative, "cs").await
        } else {
            MarkerProof::tac_cases(root.path(), &relative)
        };
        let cases = outcome.unwrap_or_else(|error| panic!("{realm}: {error}"));
        let outcomes: Vec<(&str, bool)> = cases
            .iter()
            .map(|case| (case.name.as_str(), case.passed))
            .collect();
        assert_eq!(
            outcomes,
            [
                ("testFailsAfterAnAwait", false),
                ("testPassesAfterAnAwait", true),
            ],
            "{realm}: {cases:?}"
        );
        assert!(
            cases[0].error.starts_with("after an await"),
            "{realm}: {cases:?}"
        );
    }
}
