pub(crate) struct ContextMembers<'a> {
    pub(crate) language: &'a str,
    pub(crate) tac: bool,
    pub(crate) client: &'a str,
    pub(crate) setup: &'a str,
    pub(crate) partition: &'a str,
    pub(crate) indent: &'a str,
}

impl ContextMembers<'_> {
    const STEP: &'static str = "    ";
    const FIELD_FIRST: [&'static str; 7] = ["java", "groovy", "kt", "cs", "php", "dart", "swift"];

    pub(crate) fn parts(&self) -> (String, String) {
        let lines = self.lines();
        if !Self::FIELD_FIRST.contains(&self.language) {
            return (String::new(), self.indented(&lines));
        }
        let Some((field, rest)) = lines.split_first() else {
            return (String::new(), String::new());
        };
        let rest: Vec<String> = rest
            .iter()
            .skip_while(|line| line.is_empty())
            .cloned()
            .collect();
        let text = if rest.is_empty() {
            String::new()
        } else {
            self.indented(&rest)
        };
        (format!(" {field}"), text)
    }

    fn lines(&self) -> Vec<String> {
        match self.language {
            "java" | "groovy" => self.java(),
            "kt" => self.kotlin(),
            "cs" => self.csharp(),
            "php" => self.php(),
            "dart" => self.dart(),
            "swift" => self.swift(),
            "py" => self.python(),
            "js" | "ts" => self.javascript(),
            "rb" => self.ruby(),
            "rs" => self.rust(),
            _ => Vec::new(),
        }
    }

    pub(crate) fn go(&self) -> String {
        let client = self.client;
        let create = format!(
            "\tcontext, err := BehaviorsCreate[{}](NewBehaviorOptions(\"{}\", NewBehaviorSignal()))",
            self.setup, self.partition
        );
        if self.tac {
            return format!(
                "\nfunc New{client}() (*{client}, error) {{\n{create}\n\treturn &{client}{{behaviors: context}}, err\n}}\n"
            );
        }
        format!(
            "\n// @OnStart\nfunc (client *{client}) StartContext() error {{\n{create}\n\tclient.behaviors = context\n\treturn err\n}}\n"
        )
    }

    fn indented(&self, lines: &[String]) -> String {
        let mut text = String::from("\n");
        for line in lines {
            if line.is_empty() {
                text.push('\n');
            } else {
                text.push_str(self.indent);
                text.push_str(line);
                text.push('\n');
            }
        }
        text
    }

    fn body(line: &str) -> String {
        format!("{}{line}", Self::STEP)
    }

    fn java(&self) -> Vec<String> {
        vec![
            String::from("Behaviors.Context behaviors;"),
            String::new(),
            String::from("@OnStart"),
            String::from("void startContext() {"),
            Self::body(&format!(
                "behaviors = Behaviors.create(\"{}\", new Behaviors.Options(\"{}\", new Behaviors.Signal()));",
                self.setup, self.partition
            )),
            String::from("}"),
        ]
    }

    fn kotlin(&self) -> Vec<String> {
        let create = format!(
            "Behaviors.create(\"{}\", Behaviors.Options(\"{}\", Behaviors.Signal()))",
            self.setup, self.partition
        );
        if self.tac {
            return vec![
                String::from("lateinit var behaviors: Behaviors.Context"),
                String::new(),
                String::from("init {"),
                Self::body(&format!("behaviors = {create}")),
                String::from("}"),
            ];
        }
        vec![
            String::from("lateinit var behaviors: Behaviors.Context"),
            String::new(),
            String::from("@OnStart"),
            String::from("fun startContext() {"),
            Self::body(&format!("behaviors = {create}")),
            String::from("}"),
        ]
    }

    fn csharp(&self) -> Vec<String> {
        let create = format!(
            "Behaviors.create(typeof({}), \"{}\", new BehaviorSignal())",
            self.setup, self.partition
        );
        if self.tac {
            return vec![format!("public BehaviorContext behaviors = {create};")];
        }
        vec![
            String::from("public BehaviorContext behaviors;"),
            String::new(),
            String::from("[OnStart]"),
            String::from("public void StartContext() {"),
            Self::body(&format!("behaviors = {create};")),
            String::from("}"),
        ]
    }

    fn php(&self) -> Vec<String> {
        vec![
            String::from("public BehaviorContext $behaviors;"),
            String::new(),
            String::from("#[OnStart]"),
            String::from("public function startContext(): void {"),
            Self::body(&format!(
                "$this->behaviors = Behaviors::create({}::class, ['partition' => '{}', 'signal' => new BehaviorSignal()]);",
                self.setup, self.partition
            )),
            String::from("}"),
        ]
    }

    fn dart(&self) -> Vec<String> {
        let create = format!(
            "Behaviors.create({}, partition: '{}', signal: BehaviorSignal())",
            self.setup, self.partition
        );
        if self.tac {
            return vec![format!("late final BehaviorContext behaviors = {create};")];
        }
        vec![
            String::from("late BehaviorContext behaviors;"),
            String::new(),
            String::from("@OnStart"),
            String::from("void startContext() {"),
            Self::body(&format!("behaviors = {create};")),
            String::from("}"),
        ]
    }

    fn swift(&self) -> Vec<String> {
        vec![format!(
            "let behaviors: BehaviorContext = try! Behaviors.create({}.self, partition: \"{}\", signal: BehaviorSignal())",
            self.setup, self.partition
        )]
    }

    fn python(&self) -> Vec<String> {
        vec![
            String::from("behaviors: object"),
            String::new(),
            String::from("@on_start"),
            String::from("async def start_context(self):"),
            Self::body(&format!(
                "self.behaviors = await Behaviors.create({}, {{'partition': '{}', 'signal': BehaviorSignal()}})",
                self.setup, self.partition
            )),
        ]
    }

    fn javascript(&self) -> Vec<String> {
        let typescript = self.language == "ts";
        let (field, answer) = if typescript {
            ("behaviors!: BehaviorContext;", ": Promise<void>")
        } else {
            ("behaviors;", "")
        };
        vec![
            String::from(field),
            String::new(),
            String::from("@OnStart"),
            format!("async startContext(){answer} {{"),
            Self::body(&format!(
                "this.behaviors = await Behaviors.create({}, {{ partition: '{}', signal: AbortSignal.timeout(10000) }});",
                self.setup, self.partition
            )),
            String::from("}"),
        ]
    }

    fn ruby(&self) -> Vec<String> {
        vec![
            String::from("#[OnStart]"),
            String::from("def start_context"),
            Self::body(&format!(
                "@behaviors = Behaviors.create({}, partition: \"{}\", signal: BehaviorSignal.new)",
                self.setup, self.partition
            )),
            String::from("end"),
        ]
    }

    fn rust(&self) -> Vec<String> {
        if self.setup.is_empty() {
            return vec![String::from("fn new() -> Self { Self }")];
        }
        vec![
            String::from("fn new() -> Self {"),
            Self::body(&format!(
                "let options = BehaviorOptions::new(\"{}\", BehaviorSignal::new());",
                self.partition
            )),
            Self::body(&format!(
                "Self {{ behaviors: Behaviors::create::<{}>(options).expect(\"{} creates the {} context\") }}",
                self.setup, self.setup, self.partition
            )),
            String::from("}"),
        ]
    }
}
