use super::{
    CppAttribute, CppClass, CppFailure, CppFile, CppFolder, CppLowering, CppMarkerCheck, CppMember,
    CppMemberKind, CppProgram, CppSetup, CppSource, CppTestCase,
};
use crate::Failure;
use crate::annotation_rules::Realm;
use crate::behavior::{WrittenMarker, WrittenPolicies};
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

pub(crate) struct CppTestUnit {
    pub(crate) name: String,
    pub(crate) setup: String,
    pub(crate) cases: Vec<CppTestCase>,
    pub(crate) fakes: Vec<(String, usize)>,
    pub(crate) fresh: bool,
    pub(crate) path: PathBuf,
    partition: String,
    selection: CppTestSelection,
    lowered: String,
    side: usize,
}

type CppTestSelection = (Option<Value>, Option<Map<String, Value>>);

impl CppTestUnit {
    const TESTS: [&'static str; 2] = ["client/tests", "server/tests"];
    const PARTITION: &'static str = "test";
    const FRESH_START: &'static str = "FreshStart";
    const NO_TEST: &'static str =
        "A C++ test unit declares one class marked [[Test(<Name>TestSetup)]]";
    const FAKE_FORM: &'static str = "A C++ [[Fake]] field holds the stand-in object itself, for example `[[Fake]] GreetingClient greetingClient;`: no reference, no pointer, no static";
    const FAKE_TYPES: [&'static str; 2] = [
        "A C++ [[Fake]] field stands in for a C++ service in client/services or a C++ [[HTTPClient]] in client/remotes",
        "A C++ [[Fake]] field stands in for a C++ class of the project in server/services, server/delegates, server/converters, server/stores, server/remotes or server/controllers",
    ];
    const FAKE_IN_SETUP: &'static str = "A C++ [[Fake]] field belongs in the [[Test]] class, where construct<T>() gives it to the service; the TestSetup class cannot hold one";

    pub(crate) fn read(project: &Path, relative: &Path, source: &str) -> Result<Self, Failure> {
        let mut unit = Self::unit(relative, source)?;
        let (setup, selection, fresh) = unit.read_setup(project)?;
        unit.selection = (
            unit.selection.0.take().or(selection.0),
            unit.selection.1.take().or(selection.1),
        );
        unit.fresh = [unit.fresh, fresh].contains(&true);
        unit.lowered = format!(
            "{setup}{}{}",
            CppProgram::located(&unit.path, &unit.lowered),
            unit.options()
        );
        Ok(unit)
    }

    pub(crate) fn check(relative: &Path, source: &str) -> Result<(), Failure> {
        Self::unit(relative, source).map(drop)
    }

    pub(crate) fn check_fake_types(&self, known: impl Fn(&str) -> bool) -> Result<(), Failure> {
        self.fakes.iter().try_for_each(|(type_name, at)| {
            known(type_name)
                .then_some(())
                .ok_or_else(|| CppFailure::pending(&self.path, *at, Self::FAKE_TYPES[self.side]))
        })
    }

    pub(crate) fn source(&self) -> &str {
        &self.lowered
    }

    pub(crate) fn opened(&self) -> String {
        format!(
            "tachyon::TacHost::reset(); tachyon::TacBehaviors::reset(); tachyon::TacTestAnswers answers(tachyon::Behaviors::create<{}>(tachyonTestOptions())); ",
            self.setup
        )
    }

    fn unit(relative: &Path, source: &str) -> Result<Self, Failure> {
        let file = CppFile::read(relative, source)?;
        CppMarkerCheck::check(relative, &file.class)?;
        let side = usize::from(Realm::of(relative) == Some(Realm::Yon));
        let marker = file
            .class
            .markers
            .iter()
            .find(|marker| marker.name == "Test")
            .ok_or_else(|| CppFailure::unreadable(relative, file.class.name_at, Self::NO_TEST))?;
        let setup = Self::setup_name(relative, marker, side)?;
        let written = WrittenPolicies::new(relative, source, CppSource::EXTENSION);
        let partition = Self::partition(&written, marker)?;
        let selection = Self::selection(&written, &file.class, relative)?;
        let fakes = Self::fakes(relative, &file.class.members)?;
        let fake_types = fakes
            .iter()
            .map(|member| (member.plain_type(), member.name_at))
            .collect();
        Ok(Self {
            name: file.class.name.clone(),
            setup,
            cases: CppTestCase::read_all(relative, &file.class)?,
            fakes: fake_types,
            fresh: Self::starts_fresh(&file.class),
            path: relative.to_path_buf(),
            partition,
            selection,
            lowered: CppLowering::test_unit(&file, source, &fakes),
            side,
        })
    }

    fn fakes<'a>(relative: &Path, members: &'a [CppMember]) -> Result<Vec<&'a CppMember>, Failure> {
        let fakes: Vec<&CppMember> = members
            .iter()
            .filter(|member| member.marked("Fake"))
            .collect();
        for fake in &fakes {
            let plain =
                [fake.is_instance(CppMemberKind::Field), !fake.is_reference()] == [true, true];
            plain
                .then_some(())
                .ok_or_else(|| CppFailure::unreadable(relative, fake.name_at, Self::FAKE_FORM))?;
        }
        Ok(fakes)
    }

    fn no_setup(side: usize) -> String {
        format!(
            "[[Test]] names its TestSetup class first, for example [[Test(TallyTestSetup)]], and {}/TallyTestSetup.hpp declares class [[TestSetup]] TallyTestSetup",
            Self::TESTS[side]
        )
    }

    fn setup_name(relative: &Path, marker: &CppAttribute, side: usize) -> Result<String, Failure> {
        marker
            .arguments
            .split(',')
            .next()
            .map(str::trim)
            .filter(|name| [name.is_empty(), name.starts_with('{')] == [false, false])
            .map(str::to_owned)
            .ok_or_else(|| CppFailure::unreadable(relative, marker.at, &Self::no_setup(side)))
    }

    fn partition(written: &WrittenPolicies<'_>, marker: &CppAttribute) -> Result<String, Failure> {
        let unit = written.test_unit(&Self::written(marker))?;
        Ok(unit["partition"]
            .as_str()
            .unwrap_or(Self::PARTITION)
            .to_owned())
    }

    fn selection(
        written: &WrittenPolicies<'_>,
        class: &CppClass,
        relative: &Path,
    ) -> Result<CppTestSelection, Failure> {
        let environment = CppSetup::environment(relative);
        let profiles = Self::marker(class, "UseProfiles")
            .map(|marker| written.profiles(&Self::written(marker)))
            .transpose()?;
        let properties = Self::marker(class, "TestSettings")
            .map(|marker| written.test_settings(&Self::written(marker), environment))
            .transpose()?;
        Ok((profiles, properties))
    }

    fn marker<'c>(class: &'c CppClass, name: &str) -> Option<&'c CppAttribute> {
        class.markers.iter().find(|marker| marker.name == name)
    }

    fn written(marker: &CppAttribute) -> WrittenMarker<'_> {
        WrittenMarker {
            name: &marker.name,
            arguments: &marker.arguments,
            at: marker.at,
        }
    }

    fn starts_fresh(class: &CppClass) -> bool {
        class
            .markers
            .iter()
            .any(|marker| marker.name == Self::FRESH_START)
    }

    fn read_setup(&self, project: &Path) -> Result<(String, CppTestSelection, bool), Failure> {
        let path = project
            .join(Self::TESTS[self.side])
            .join(format!("{}.hpp", self.setup));
        let (relative, source) = CppFolder::read(project, &path)
            .map_err(|_| CppFailure::unreadable(&self.path, 0, &Self::no_setup(self.side)))?;
        let file = CppFile::read(&relative, &source)?;
        CppMarkerCheck::check(&relative, &file.class)?;
        let marked = file
            .class
            .markers
            .iter()
            .any(|marker| marker.name == "TestSetup");
        let valid = [marked, file.class.name == self.setup] == [true, true];
        valid.then_some(()).ok_or_else(|| {
            CppFailure::unreadable(&relative, file.class.name_at, &Self::no_setup(self.side))
        })?;
        let fake = file
            .class
            .members
            .iter()
            .find(|member| member.marked("Fake"));
        fake.map_or(Ok(()), |fake| {
            Err(CppFailure::unreadable(
                &relative,
                fake.name_at,
                Self::FAKE_IN_SETUP,
            ))
        })?;
        let written = WrittenPolicies::new(&relative, &source, CppSource::EXTENSION);
        let selection = Self::selection(&written, &file.class, &relative)?;
        let plan = CppSetup::plan(&relative, &source, &file.class)?;
        let lowered = CppLowering::model(&file, &source);
        let located = CppProgram::located(&relative, &lowered);
        Ok((
            format!("{located}{}{plan}", CppProgram::GENERATED),
            selection,
            Self::starts_fresh(&file.class),
        ))
    }

    fn options(&self) -> String {
        let listed: Vec<String> = self
            .selection
            .0
            .as_ref()
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(Value::to_string)
            .collect();
        let pairs: Vec<String> = self
            .selection
            .1
            .iter()
            .flatten()
            .map(|(name, value)| format!("{{{}, {value}}}", Value::String(name.clone())))
            .collect();
        format!(
            "#line 1 \"tachyon-test-main.cpp\"\nstatic tachyon::BehaviorOptions tachyonTestOptions() {{\n    tachyon::BehaviorOptions options;\n    options.partition = {};\n    options.profiles = {{{}}};\n    options.properties = {{{}}};\n    return options;\n}}\n",
            Value::String(self.partition.clone()),
            listed.join(", "),
            pairs.join(", ")
        )
    }
}
