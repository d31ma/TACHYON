use super::{CppClass, CppFailure, CppFile, CppListener, CppMarkerCheck, CppMember, CppMemberKind};
use crate::Failure;
use serde_json::{Value, json};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

pub(crate) struct CppService {
    pub(crate) file: CppFile,
    pub(crate) path: PathBuf,
    pub(crate) source: String,
    pub(crate) listeners: Vec<CppListener>,
}

impl CppService {
    const PUBLISHER: &'static str = "ApplicationEventPublisher";
    const LIFECYCLE: [&'static str; 3] = ["OnEvent", "OnStart", "OnStop"];
    const NEEDS_FORM: &'static str = "A C++ [[Needs]] field is a reference with no initial value, for example `[[Needs]] OrderStore& orders;`";
    const NESTED: &'static str =
        "A C++ service declares no nested types; put a model in client/models/<Name>.hpp";
    const OVERLOADED: &'static str = "A C++ service method that the page calls has one name; C++ overloads cannot reach the page. Rename one of the methods";
    const NAMED: &'static str = "A C++ class is named after its file";

    pub(crate) fn read(relative: &Path, source: &str) -> Result<Self, Failure> {
        let file = CppFile::read(relative, source)?;
        let class = &file.class;
        let named = relative
            .file_stem()
            .is_some_and(|stem| stem == class.name.as_str());
        if !named {
            return Err(CppFailure::unreadable(
                relative,
                class.name_at,
                &format!(
                    "{}: rename the class or the file so both say {}",
                    Self::NAMED,
                    relative
                        .file_stem()
                        .map(|stem| stem.to_string_lossy())
                        .unwrap_or_default()
                ),
            ));
        }
        CppMarkerCheck::check(relative, class)?;
        Self::check_members(relative, class)?;
        let listeners = class
            .members
            .iter()
            .filter(|member| member.marked("OnEvent"))
            .map(|member| CppListener::read(relative, member))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            file,
            path: relative.to_path_buf(),
            source: source.to_owned(),
            listeners,
        })
    }

    fn check_members(relative: &Path, class: &CppClass) -> Result<(), Failure> {
        for member in &class.members {
            if member.kind == CppMemberKind::NestedType {
                return Err(CppFailure::unreadable(
                    relative,
                    member.name_at,
                    Self::NESTED,
                ));
            }
            let needs_wrong = member.marked("Needs")
                && (!member.type_text.ends_with('&')
                    || member.has_initializer
                    || member.declarators > 1);
            if needs_wrong {
                return Err(CppFailure::unreadable(
                    relative,
                    member.name_at,
                    Self::NEEDS_FORM,
                ));
            }
            if member.kind == CppMemberKind::Field && member.declarators > 1 {
                return Err(CppFailure::unreadable(
                    relative,
                    member.name_at,
                    "Declare one C++ field per line",
                ));
            }
        }
        let methods: Vec<&str> = Self::page_methods_of(class)
            .map(|member| member.name.as_str())
            .collect();
        let repeated = methods
            .iter()
            .enumerate()
            .find(|(index, name)| methods[..*index].contains(name));
        if let Some((_, name)) = repeated {
            let at = class
                .members
                .iter()
                .rev()
                .find(|member| member.name == *name)
                .map_or(0, |member| member.name_at);
            return Err(CppFailure::unreadable(relative, at, Self::OVERLOADED));
        }
        Ok(())
    }

    pub(crate) fn class(&self) -> &CppClass {
        &self.file.class
    }

    pub(crate) fn needs(&self) -> Vec<(String, String)> {
        self.class()
            .members
            .iter()
            .filter(|member| member.marked("Needs"))
            .map(|member| (member.name.clone(), member.plain_type()))
            .collect()
    }

    pub(crate) fn page_fields(&self) -> Vec<&CppMember> {
        self.class()
            .members
            .iter()
            .filter(|member| {
                member.kind == CppMemberKind::Field && member.public && !member.is_static
            })
            .filter(|member| !member.marked("Needs") && !member.is_reference())
            .filter(|member| member.plain_type() != Self::PUBLISHER)
            .collect()
    }

    pub(crate) fn page_methods(&self) -> Vec<&CppMember> {
        Self::page_methods_of(self.class()).collect()
    }

    fn page_methods_of(class: &CppClass) -> impl Iterator<Item = &CppMember> {
        class
            .members
            .iter()
            .filter(|member| {
                member.kind == CppMemberKind::Method && member.public && !member.is_static
            })
            .filter(|member| !Self::LIFECYCLE.iter().any(|name| member.marked(name)))
    }

    fn hooks(&self, marker: &str) -> Vec<String> {
        self.class()
            .members
            .iter()
            .filter(|member| member.kind == CppMemberKind::Method && member.marked(marker))
            .map(|member| member.name.clone())
            .collect()
    }

    pub(crate) fn binding_name(&self) -> String {
        format!("TachyonBinding{}", self.class().name)
    }

    pub(crate) fn metadata(&self) -> Value {
        let fields: Vec<String> = self
            .page_fields()
            .iter()
            .map(|member| Self::page_name(&member.name))
            .collect();
        let methods: Vec<String> = self
            .page_methods()
            .iter()
            .map(|member| Self::page_name(&member.name))
            .collect();
        json!({
            "page": {
                "fields": fields,
                "methods": methods,
                "listeners": self.listeners.iter().map(CppListener::page).collect::<Vec<_>>(),
            },
            "starts": self.hooks("OnStart"),
            "stops": self.hooks("OnStop"),
            "listeners": self.listeners.iter().map(CppListener::program).collect::<Vec<_>>(),
        })
    }

    fn page_name(name: &str) -> String {
        crate::companion::camel_case(name)
    }

    pub(crate) fn binding(&self) -> String {
        let class = &self.class().name;
        let mut fields = String::new();
        for member in self.page_fields() {
            let _ = write!(
                fields,
                "tachyon::TacBind::field(\"{}\", &{class}::{}), ",
                Self::page_name(&member.name),
                member.name
            );
        }
        let mut methods = String::new();
        for member in self.page_methods() {
            let _ = write!(
                methods,
                "tachyon::TacBind::method(\"{}\", &{class}::{}), ",
                Self::page_name(&member.name),
                member.name
            );
        }
        let mut hooks = String::new();
        let hooked = self
            .listeners
            .iter()
            .map(|listener| listener.method.clone())
            .chain(self.hooks("OnStart"))
            .chain(self.hooks("OnStop"));
        for name in hooked {
            let _ = write!(
                hooks,
                "tachyon::TacBind::method(\"{name}\", &{class}::{name}), "
            );
        }
        let binding = self.binding_name();
        let metadata = self.metadata();
        format!(
            "class {binding} {{\npublic:\n    static tachyon::TacServiceType type() {{\n        tachyon::TacServiceType type;\n        type.metadata = tachyon::json::JsonNode::parse(R\"tachyon({metadata})tachyon\");\n        type.create = [] {{ return std::shared_ptr<void>(&tachyon::TacNeeds::of<{class}>(), [](void*) {{}}); }};\n        type.fields = {{ {fields}}};\n        type.methods = {{ {methods}}};\n        type.hooks = {{ {hooks}}};\n        return type;\n    }}\n}};\n"
        )
    }
}
