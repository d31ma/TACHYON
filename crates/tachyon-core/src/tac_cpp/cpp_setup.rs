use super::{
    CppAttribute, CppClass, CppFailure, CppFile, CppFolder, CppLowering, CppMarkerCheck, CppMember,
    CppMemberKind, CppProgram, CppSettings, CppSource, CppSpelling,
};
use crate::Failure;
use crate::annotation_rules::Realm;
use crate::behavior::{WrittenMarker, WrittenPolicies};
use serde_json::{Map, Value};
use std::path::Path;

pub(crate) struct CppSetup;

impl CppSetup {
    const FOLDERS: [&'static str; 2] = ["client/setups", "server/setups"];
    const OPENERS: [&'static str; 2] = ["a service", "a controller, service or helper"];
    const ENVIRONMENTS: [&'static str; 2] = ["tac", "yon"];
    const SETUP: &'static str = "Setup";
    const CONFIGURE: &'static str = "configure";
    pub(crate) const ENVIRONMENT: &'static str = "tac";
    const CLASS_MARKERS: [&'static str; 4] = ["Setup", "WhenProfile", "WhenSetting", "Description"];
    const NO_CONFIGURE: &'static str = "A C++ [[Setup]] declares `static BehaviorAdapters configure(const BehaviorExecution& execution)`, which returns the adapters of its behavior context";
    const OTHER_MARKER: &'static str = "A C++ [[Setup]] class takes only [[WhenProfile]], [[WhenSetting]] and [[Description]] next to [[Setup]]; it has no other role";
    const MEMBER_MARKER: &'static str = "A C++ [[Setup]] class has no member markers: its static configure method returns the adapters, and Behaviors::create<Setup>(options) opens a context from it";

    pub(crate) fn read_all(root: &Path, realm: Realm) -> Result<Vec<String>, Failure> {
        CppFolder::files(root, Self::FOLDERS[Self::index(realm)])?
            .iter()
            .map(|path| Self::read(root, path))
            .collect()
    }

    pub(crate) fn plan(path: &Path, source: &str, class: &CppClass) -> Result<String, Failure> {
        let written = WrittenPolicies::new(path, source, CppSource::EXTENSION);
        let environment = Self::environment(path);
        let conditions = class
            .markers
            .iter()
            .map(|marker| Self::condition(&written, marker, environment))
            .collect::<Result<Vec<_>, _>>()?;
        let mut plan: Map<String, Value> = conditions.into_iter().flatten().collect();
        plan.insert(String::from("name"), Value::String(class.name.clone()));
        Ok(CppSettings::specialization(
            "TacSetupPlan",
            &class.name,
            &Value::Object(plan),
        ))
    }

    pub(crate) fn refuse_elsewhere(path: &Path, class: &CppClass) -> Result<(), Failure> {
        class
            .markers
            .iter()
            .find(|marker| marker.name == Self::SETUP)
            .map_or(Ok(()), |marker| {
                Err(CppFailure::unreadable(
                    path,
                    marker.at,
                    &Self::elsewhere(Self::index(Self::realm(path))),
                ))
            })
    }

    pub(crate) fn environment(path: &Path) -> &'static str {
        Self::ENVIRONMENTS[Self::index(Self::realm(path))]
    }

    fn realm(path: &Path) -> Realm {
        Realm::of(path).unwrap_or(Realm::Tac)
    }

    fn index(realm: Realm) -> usize {
        usize::from(realm == Realm::Yon)
    }

    fn elsewhere(index: usize) -> String {
        format!(
            "[[Setup]] goes on a class in {folder}/<Name>.hpp, for example `class [[Setup]] CatalogSetup`, and {opener} opens a context with Behaviors::create<CatalogSetup>(options)",
            folder = Self::FOLDERS[index],
            opener = Self::OPENERS[index]
        )
    }

    fn no_setup(path: &Path) -> String {
        let folder = Self::FOLDERS[Self::index(Self::realm(path))];
        format!(
            "A C++ file in {folder} declares one class marked [[Setup]] and named after the file, for example `class [[Setup]] CatalogSetup` in {folder}/CatalogSetup.hpp"
        )
    }

    fn read(root: &Path, path: &Path) -> Result<String, Failure> {
        let (relative, source) = CppFolder::read(root, path)?;
        let file = CppFile::read(&relative, &source)?;
        CppMarkerCheck::check(&relative, &file.class)?;
        Self::check_name(&relative, &file.class)?;
        Self::check_markers(&relative, &file.class)?;
        Self::check_configure(&relative, &file.class)?;
        let plan = Self::plan(&relative, &source, &file.class)?;
        let lowered = CppLowering::model(&file, &source);
        Ok(format!(
            "{}{}{plan}",
            CppProgram::located(&relative, &lowered),
            CppProgram::GENERATED
        ))
    }

    fn condition(
        written: &WrittenPolicies<'_>,
        marker: &CppAttribute,
        environment: &str,
    ) -> Result<Option<(String, Value)>, Failure> {
        let written_marker = WrittenMarker {
            name: &marker.name,
            arguments: &marker.arguments,
            at: marker.at,
        };
        match marker.name.as_str() {
            "WhenProfile" => written
                .profiles(&written_marker)
                .map(|value| Some((String::from("profiles"), value))),
            "WhenSetting" => written
                .condition(&written_marker, environment)
                .map(|value| Some((String::from("conditionalOnProperty"), value))),
            _ => Ok(None),
        }
    }

    fn check_name(path: &Path, class: &CppClass) -> Result<(), Failure> {
        let named = path
            .file_stem()
            .is_some_and(|stem| stem == class.name.as_str());
        let marked = class
            .markers
            .iter()
            .any(|marker| marker.name == Self::SETUP);
        ([named, marked] == [true, true])
            .then_some(())
            .ok_or_else(|| CppFailure::unreadable(path, class.name_at, &Self::no_setup(path)))
    }

    fn check_markers(path: &Path, class: &CppClass) -> Result<(), Failure> {
        let other = class
            .markers
            .iter()
            .filter(|marker| Self::is_tachyon(marker))
            .find(|marker| !Self::CLASS_MARKERS.contains(&marker.name.as_str()));
        other.map_or(Ok(()), |marker| {
            Err(CppFailure::unreadable(path, marker.at, Self::OTHER_MARKER))
        })?;
        let marked = class
            .members
            .iter()
            .flat_map(|member| member.markers.iter())
            .find(|marker| Self::is_tachyon(marker));
        marked.map_or(Ok(()), |marker| {
            Err(CppFailure::unreadable(path, marker.at, Self::MEMBER_MARKER))
        })
    }

    fn is_tachyon(marker: &CppAttribute) -> bool {
        CppSpelling::of(&marker.name) == CppSpelling::Exact
    }

    fn check_configure(path: &Path, class: &CppClass) -> Result<(), Failure> {
        class
            .members
            .iter()
            .any(Self::is_configure)
            .then_some(())
            .ok_or_else(|| CppFailure::unreadable(path, class.name_at, Self::NO_CONFIGURE))
    }

    fn is_configure(member: &CppMember) -> bool {
        [
            member.kind == CppMemberKind::Method,
            member.is_static,
            member.name == Self::CONFIGURE,
        ] == [true, true, true]
    }
}
