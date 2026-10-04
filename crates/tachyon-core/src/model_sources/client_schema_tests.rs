#![allow(clippy::expect_used)]

use std::path::Path;

struct ClientSchema {
    extension: &'static str,
    model: &'static str,
    service: &'static str,
}

impl ClientSchema {
    const CASES: [Self; 4] = [
        Self {
            extension: "cs",
            model: "[Checked]\npublic sealed class OrderInput {\n    [NotEmpty]\n    [Length({\"min\":1,\"max\":80})]\n    public BehaviorJSON name;\n    [Required]\n    [Min(1)]\n    public BehaviorJSON level;\n}\n",
            service: "using Models;\n\n[Service]\npublic class OrderService {\n    public BehaviorJSON accept([Check(typeof(OrderInput))] BehaviorJSON order) { return order; }\n}\n",
        },
        Self {
            extension: "rs",
            model: "#[Checked]\npub struct OrderInput {\n    #[NotEmpty]\n    #[Length({\"min\":1,\"max\":80})]\n    pub name: String,\n    #[Required]\n    #[Min(1)]\n    level: f64,\n}\n",
            service: "#[Service]\nstruct OrderService {}\nimpl OrderService {\n    fn new() -> Self { Self {} }\n    fn accept(&self, #[Check(OrderInput)] order: BehaviorValue) -> BehaviorResult<BehaviorValue> { Ok(order) }\n}\n",
        },
        Self {
            extension: "kt",
            model: "@Checked\nclass OrderInput {\n    @Required\n    @Min(1)\n    var level: Any? = null\n}\n",
            service: "import models.OrderInput\n\n@Service\nclass OrderService {\n    fun accept(@Check(OrderInput::class) order: Any?): Any? { return order }\n}\n",
        },
        Self {
            extension: "swift",
            model: "@Checked\nstruct OrderInput {\n    @Required\n    @Min(1)\n    var level: Double = 0\n}\n",
            service: "@Service\nfinal class OrderService {\n    func accept(@Check(OrderInput) _ order: BehaviorJSON) throws -> BehaviorJSON { return order }\n}\n",
        },
    ];

    fn declares_input(lowered: &str) -> bool {
        ["class OrderInput", "struct OrderInput"]
            .iter()
            .any(|declaration| lowered.contains(declaration))
    }

    fn lowered(&self) -> Result<String, String> {
        let root = tempfile::tempdir().expect("project");
        let model = root
            .path()
            .join(format!("client/models/OrderInput.{}", self.extension));
        std::fs::create_dir_all(model.parent().expect("models")).expect("folder");
        std::fs::write(&model, self.model).expect("model");
        let service = format!("client/services/OrderService.{}", self.extension);
        let path = Path::new(&service);
        let mut assembled = String::new();
        let lowered = super::ModelSources::client_metadata(root.path(), self.extension).and_then(
            |metadata| {
                metadata.lowered(path, self.service, |text| {
                    assembled = text.to_owned();
                    let roles = crate::behavior::ServiceRoles::read(path, text)?;
                    crate::behavior::lower(path, &roles.source, "tac")
                })
            },
        );
        match lowered {
            Ok(Some(lowered)) => Ok(lowered),
            Ok(None) => Err(format!("{}: nothing lowered", self.extension)),
            Err(failure) => Err(format!("{}: {failure}\n{assembled}", self.extension)),
        }
    }
}

#[test]
fn compiled_tac_services_check_input_against_a_schema_model() {
    let mut failures = Vec::new();
    for case in ClientSchema::CASES {
        match case.lowered() {
            Ok(lowered) if ClientSchema::declares_input(&lowered) => {
                failures.push(format!("{}: the schema declaration stayed", case.extension));
            }
            Ok(_) => {}
            Err(failure) => failures.push(failure),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n----\n"));
}
