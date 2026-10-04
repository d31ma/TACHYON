#[derive(Clone, Copy)]
pub(crate) struct ContextTemplate {
    pub(crate) key: &'static str,
    pub(crate) field: &'static str,
    pub(crate) members: &'static str,
}

impl ContextTemplate {
    const JAVA_FIELD: &'static str = "Behaviors.Context behaviors;";
    const JAVA_START: &'static str = "@OnStart\nvoid startContext() {\n    behaviors = Behaviors.create(\"{setup}\", new Behaviors.Options(\"{partition}\", new Behaviors.Signal()));\n}";
    const KOTLIN_FIELD: &'static str = "lateinit var behaviors: Behaviors.Context";
    const DART_TAC_HTTP: Self = Self::new(
        "dart-tac",
        "final BehaviorContext? _suppliedContext;\nlate final BehaviorContext behaviors = _suppliedContext ?? Behaviors.create({setup}, partition: '{partition}', signal: BehaviorSignal());",
        "{owner}([this._suppliedContext]);",
    );
    const CSHARP_TAC_HTTP: Self = Self::new(
        "cs-tac",
        "private readonly BehaviorContext behaviors;",
        "public {owner}() : this(Behaviors.create(typeof({setup}), \"{partition}\", new BehaviorSignal())) {}\npublic {owner}(BehaviorContext context) {\n    behaviors = context ?? throw new System.ArgumentNullException(nameof(context));\n}",
    );
    const RUST_TAC_HTTP: Self = Self::new(
        "rs-tac",
        "",
        "fn new() -> Self {\n    let options = BehaviorOptions::new(\"{partition}\", BehaviorSignal::new());\n    Self { behaviors: Behaviors::create::<{setup}>(options).expect(\"{setup} creates the {partition} context\") }\n}\npub fn with_context(behaviors: BehaviorContext) -> Self {\n    Self { behaviors }\n}",
    );
    const GO_TAC_HTTP: Self = Self::new(
        "go-tac",
        "",
        "func New{owner}() (*{owner}, error) {\n\tcontext, err := BehaviorsCreate[{setup}](NewBehaviorOptions(\"{partition}\", NewBehaviorSignal()))\n\treturn &{owner}{behaviors: context}, err\n}\n\nfunc New{owner}With(context *BehaviorContext) *{owner} {\n\treturn &{owner}{behaviors: context}\n}",
    );
    const KOTLIN_TAC_HTTP: Self = Self::new(
        "kt-tac",
        "val behaviors: Behaviors.Context",
        "constructor() {\n    behaviors = Behaviors.create(\"{setup}\", Behaviors.Options(\"{partition}\", Behaviors.Signal()))\n}\nconstructor(context: Behaviors.Context) {\n    behaviors = context\n}",
    );
    const HTTP_TEMPLATES: [Self; 5] = [
        Self::DART_TAC_HTTP,
        Self::CSHARP_TAC_HTTP,
        Self::RUST_TAC_HTTP,
        Self::GO_TAC_HTTP,
        Self::KOTLIN_TAC_HTTP,
    ];
    const HTTP_CLIENT: &'static str = "HTTPClient";
    const TEMPLATES: [Self; 23] = [
        Self::new("java", Self::JAVA_FIELD, Self::JAVA_START),
        Self::new(
            "java-tac",
            Self::JAVA_FIELD,
            "{owner}() {\n    behaviors = Behaviors.create(\"{setup}\", new Behaviors.Options(\"{partition}\", new Behaviors.Signal()));\n}",
        ),
        Self::new("groovy", Self::JAVA_FIELD, Self::JAVA_START),
        Self::new(
            "kt-yon",
            Self::KOTLIN_FIELD,
            "@OnStart\nfun startContext() {\n    behaviors = Behaviors.create(\"{setup}\", Behaviors.Options(\"{partition}\", Behaviors.Signal()))\n}",
        ),
        Self::new(
            "kt-tac",
            Self::KOTLIN_FIELD,
            "init {\n    behaviors = Behaviors.create(\"{setup}\", Behaviors.Options(\"{partition}\", Behaviors.Signal()))\n}",
        ),
        Self::new(
            "cs-yon",
            "public BehaviorContext behaviors;",
            "[OnStart]\npublic void StartContext() {\n    behaviors = Behaviors.create(typeof({setup}), \"{partition}\", new BehaviorSignal());\n}",
        ),
        Self::new(
            "cs-tac",
            "public BehaviorContext behaviors = Behaviors.create(typeof({setup}), \"{partition}\", new BehaviorSignal());",
            "",
        ),
        Self::new(
            "php",
            "public BehaviorContext $behaviors;",
            "#[OnStart]\npublic function startContext(): void {\n    $this->behaviors = Behaviors::create({setup}::class, ['partition' => '{partition}', 'signal' => new BehaviorSignal()]);\n}",
        ),
        Self::new(
            "dart-yon",
            "late BehaviorContext behaviors;",
            "@OnStart\nvoid startContext() {\n    behaviors = Behaviors.create({setup}, partition: '{partition}', signal: BehaviorSignal());\n}",
        ),
        Self::new(
            "dart-tac",
            "late final BehaviorContext behaviors = Behaviors.create({setup}, partition: '{partition}', signal: BehaviorSignal());",
            "",
        ),
        Self::new(
            "swift",
            "let behaviors: BehaviorContext = try! Behaviors.create({setup}.self, partition: \"{partition}\", signal: BehaviorSignal())",
            "",
        ),
        Self::new(
            "py",
            "",
            "behaviors: object\n\n@on_start\nasync def start_context(self):\n    self.behaviors = await Behaviors.create({setup}, {'partition': '{partition}', 'signal': BehaviorSignal()})",
        ),
        Self::new(
            "js",
            "",
            "behaviors;\n\n@OnStart\nasync startContext() {\n    this.behaviors = await Behaviors.create({setup}, { partition: '{partition}', signal: AbortSignal.timeout(10000) });\n}",
        ),
        Self::new("js-tac", "", "@Needs\nbehaviors;"),
        Self::new(
            "js-provided",
            "",
            "static tachyonContext(signal) {\n    return Behaviors.create({setup}, { partition: '{partition}', signal });\n}",
        ),
        Self::new(
            "ts-provided",
            "",
            "static tachyonContext(signal: AbortSignal): Promise<BehaviorContext> {\n    return Behaviors.create({setup}, { partition: '{partition}', signal });\n}",
        ),
        Self::new("ts-tac", "", "@Needs\nbehaviors!: BehaviorContext;"),
        Self::new(
            "ts",
            "",
            "behaviors!: BehaviorContext;\n\n@OnStart\nasync startContext(): Promise<void> {\n    this.behaviors = await Behaviors.create({setup}, { partition: '{partition}', signal: AbortSignal.timeout(10000) });\n}",
        ),
        Self::new(
            "rb",
            "",
            "#[OnStart]\ndef start_context\n    @behaviors = Behaviors.create({setup}, partition: \"{partition}\", signal: BehaviorSignal.new)\nend",
        ),
        Self::new(
            "rb-tac",
            "",
            "def initialize\n    @behaviors = Behaviors.create({setup}, partition: \"{partition}\", signal: BehaviorSignal.new)\nend",
        ),
        Self::new(
            "rs",
            "",
            "fn new() -> Self {\n    let options = BehaviorOptions::new(\"{partition}\", BehaviorSignal::new());\n    Self { behaviors: Behaviors::create::<{setup}>(options).expect(\"{setup} creates the {partition} context\") }\n}",
        ),
        Self::new("rs-bare", "", "fn new() -> Self { Self }"),
        Self::new(
            "lua",
            "",
            "---@return nil\n-- @OnStart\nfunction {owner}:start_context()\n  self.behaviors = Behaviors.create({setup}, { partition = \"{partition}\", signal = BehaviorSignal.new() })\nend",
        ),
    ];
    const GO_TEMPLATES: [Self; 3] = [
        Self::new(
            "go-yon",
            "",
            "// @OnStart\nfunc ({receiver} *{owner}) StartContext() error {\n\tcontext, err := BehaviorsCreate[{setup}](NewBehaviorOptions(\"{partition}\", NewBehaviorSignal()))\n\t{receiver}.behaviors = context\n\treturn err\n}",
        ),
        Self::new(
            "go-tac",
            "",
            "func New{owner}() (*{owner}, error) {\n\tcontext, err := BehaviorsCreate[{setup}](NewBehaviorOptions(\"{partition}\", NewBehaviorSignal()))\n\treturn &{owner}{behaviors: context}, err\n}",
        ),
        Self::new(
            "go-bare",
            "",
            "func New{owner}() (*{owner}, error) {\n\treturn &{owner}{}, nil\n}",
        ),
    ];
    const EMPTY: Self = Self::new("", "", "");

    const fn new(key: &'static str, field: &'static str, members: &'static str) -> Self {
        Self {
            key,
            field,
            members,
        }
    }

    pub(crate) fn find(keys: &[String]) -> Self {
        Self::TEMPLATES
            .into_iter()
            .chain(Self::GO_TEMPLATES)
            .filter(|template| keys.contains(&template.key.to_owned()))
            .min_by_key(|template| keys.iter().position(|key| key == template.key))
            .unwrap_or(Self::EMPTY)
    }

    pub(crate) fn for_contract(contract: &str, keys: &[String]) -> Self {
        Self::http_template(contract, keys).unwrap_or_else(|| Self::find(keys))
    }

    fn http_template(contract: &str, keys: &[String]) -> Option<Self> {
        Self::http_template_for(keys).filter(|_| contract == Self::HTTP_CLIENT)
    }

    fn http_template_for(keys: &[String]) -> Option<Self> {
        Self::HTTP_TEMPLATES
            .into_iter()
            .find(|template| keys.contains(&template.key.to_owned()))
    }
}
