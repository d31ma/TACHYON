#![allow(clippy::expect_used)]
use super::{Prepared, prepare};
use serde_json::Value;
use std::path::Path;

struct DescribedSource {
    extension: &'static str,
    source: &'static str,
    in_program: bool,
}

impl DescribedSource {
    const TEXT: &'static str = "Prices one item and keeps the price";

    const SOURCES: [Self; 10] = [
        Self {
            extension: "js",
            source: "@Description(\"Prices one item and keeps the price\")\n@Service\nexport default class PriceService {\n  async price(sku) { return sku; }\n}\n",
            in_program: true,
        },
        Self {
            extension: "ts",
            source: "@Description(\"Prices one item and keeps the price\")\n@Service\nexport default class PriceService {\n  async price(sku: string) { return sku; }\n}\n",
            in_program: true,
        },
        Self {
            extension: "dart",
            source: "@Description(\"Prices one item and keeps the price\")\n@Service\nclass PriceService {\n  int reads = 0;\n\n  Future<Object?> price(String sku) async {\n    return sku;\n  }\n}\n",
            in_program: true,
        },
        Self {
            extension: "rs",
            source: "#[Description(\"Prices one item and keeps the price\")]\n#[Service]\npub struct PriceService {}\nimpl PriceService {\n    pub fn price(&self, sku: BehaviorValue) -> BehaviorResult<BehaviorValue> {\n        Ok(sku)\n    }\n}\n",
            in_program: false,
        },
        Self {
            extension: "cs",
            source: "[Description(\"Prices one item and keeps the price\")]\n[Service]\npublic sealed class PriceService {\n    public async System.Threading.Tasks.Task<BehaviorJSON> price(BehaviorJSON sku) {\n        await System.Threading.Tasks.Task.CompletedTask;\n        return sku;\n    }\n}\n",
            in_program: true,
        },
        Self {
            extension: "kt",
            source: "@Description(\"Prices one item and keeps the price\")\n@Service\nclass PriceService() {\n    fun price(sku: String): Any? {\n        return sku\n    }\n}\n",
            in_program: false,
        },
        Self {
            extension: "swift",
            source: "@Description(\"Prices one item and keeps the price\")\n@Service\nfinal class PriceService {\n    func price(_ sku: String) async throws -> BehaviorJSON {\n        return .string(sku)\n    }\n}\n",
            in_program: true,
        },
        Self {
            extension: "go",
            source: "// @Description(\"Prices one item and keeps the price\")\n// @Service\ntype PriceService struct{}\n\nfunc (prices *PriceService) Price(sku BehaviorValue) (BehaviorValue, error) {\n\treturn sku, nil\n}\n",
            in_program: false,
        },
        Self {
            extension: "rb",
            source: "#[Description(\"Prices one item and keeps the price\")]\n#[Service]\nclass PriceService\n  # @param sku [String]\n  # @return [String]\n  def price(sku)\n    sku\n  end\nend\n",
            in_program: true,
        },
        Self {
            extension: "py",
            source: "@description(\"Prices one item and keeps the price\")\n@service\nclass PriceService:\n    def price(self, sku: str) -> str:\n        return sku\n",
            in_program: true,
        },
    ];

    fn path(&self) -> String {
        format!("client/services/PriceService.{}", self.extension)
    }

    fn project(&self) -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("project");
        let file = root.path().join(self.path());
        std::fs::create_dir_all(file.parent().expect("folder")).expect("folder");
        std::fs::write(&file, self.source).expect("source");
        root
    }

    fn assembled(&self) -> String {
        let root = self.project();
        let path = self.path();
        crate::annotation_modules::assemble(root.path(), Path::new(&path), self.source, "tac")
            .unwrap_or_else(|error| panic!("{}: {error:?}", self.extension))
            .unwrap_or_else(|| self.source.to_owned())
    }

    fn native(&self) -> String {
        let assembled = self.assembled();
        if ["js", "ts"].contains(&self.extension) {
            return assembled;
        }
        let path = self.path();
        crate::behavior::ServiceRoles::read(Path::new(&path), &assembled)
            .unwrap_or_else(|error| panic!("{}: {error:?}", self.extension))
            .source
    }

    fn registered(&self) -> Value {
        let path = self.path();
        let prepared = prepare(Path::new(&path), &self.native(), "tac")
            .unwrap_or_else(|error| panic!("{}: {error:?}", self.extension));
        let Some(Prepared::Annotated(module)) = prepared else {
            panic!("{}: the service has no registration", self.extension);
        };
        let service = module
            .types
            .iter()
            .find(|ty| ty.name == "PriceService")
            .unwrap_or_else(|| panic!("{}: no PriceService type", self.extension));
        service
            .policies
            .get("description")
            .cloned()
            .unwrap_or(Value::Null)
    }

    fn lowered(&self) -> String {
        let path = self.path();
        crate::behavior::lower(Path::new(&path), &self.native(), "tac")
            .unwrap_or_else(|error| panic!("{}: {error:?}", self.extension))
            .unwrap_or_else(|| panic!("{}: no lowered program", self.extension))
    }

    fn dart_program(&self) -> String {
        let root = self.project();
        let path = self.path();
        crate::tac_dart::prepare(root.path(), Path::new(&path), self.source)
            .unwrap_or_else(|error| panic!("{}: {error:?}", self.extension))
            .source
    }

    fn program(&self) -> String {
        let dart = self.extension == "dart";
        [Self::lowered, Self::dart_program][usize::from(dart)](self)
    }

    fn registered_text(&self) -> Value {
        let carried = ["js", "ts", "dart"].contains(&self.extension);
        [Self::registered, Self::program_registration][usize::from(carried)](self)
    }

    fn program_registration(&self) -> Value {
        let program = self.program().replace("\\\"", "\"");
        let quoted = serde_json::to_string(Self::TEXT).expect("quoted text");
        let pair = format!("\"description\":{quoted}");
        Value::from(program.contains(&pair).then_some(Self::TEXT))
    }
}

#[test]
fn a_tac_description_reaches_the_registration_in_every_behavior_language() {
    for source in DescribedSource::SOURCES {
        assert_eq!(
            source.registered_text(),
            DescribedSource::TEXT,
            "{}",
            source.extension
        );
    }
}

#[test]
fn a_tac_description_is_carried_into_the_lowered_program_with_its_registration() {
    for source in DescribedSource::SOURCES {
        let program = source.program();
        assert_eq!(
            program.matches(DescribedSource::TEXT).count(),
            usize::from(source.in_program),
            "{}: {program}",
            source.extension
        );
    }
}
