use super::{CppAttribute, CppClass, CppFailure, CppFile, CppMemberKind, CppSource};
use crate::Failure;
use crate::annotation_rules::{AnnotationRules, PlacedMarker, Realm, Target};
use std::path::Path;

pub(crate) struct CppMarkerCheck<'a> {
    path: &'a Path,
    realm: Realm,
    class_markers: Vec<String>,
}

impl<'a> CppMarkerCheck<'a> {
    const STANDARD: [&'static str; 10] = [
        "noreturn",
        "carries_dependency",
        "deprecated",
        "fallthrough",
        "nodiscard",
        "maybe_unused",
        "likely",
        "unlikely",
        "no_unique_address",
        "assume",
    ];
    const OWN_NAMESPACES: [&'static str; 3] = ["tachyon", "tac", "yon"];
    const NEAR_MISS: usize = 2;

    pub(crate) fn check_source(path: &'a Path, source: &str) -> Result<(), Failure> {
        let file = CppFile::read(path, source)?;
        Self::check(path, &file.class)
    }

    pub(crate) fn check(path: &'a Path, class: &CppClass) -> Result<(), Failure> {
        let check = Self {
            path,
            realm: Realm::of(path).unwrap_or(Realm::Tac),
            class_markers: class
                .markers
                .iter()
                .map(|marker| marker.name.clone())
                .collect(),
        };
        check.declaration(&class.markers, Target::Type)?;
        for member in &class.members {
            check.declaration(&member.markers, Self::member_target(member.kind))?;
            for parameter in &member.parameters {
                check.declaration(&parameter.markers, Target::Parameter)?;
            }
        }
        Ok(())
    }

    pub(crate) fn is_standard(name: &str) -> bool {
        Self::STANDARD.contains(&name)
    }

    fn member_target(kind: CppMemberKind) -> Target {
        match kind {
            CppMemberKind::Field => Target::Field,
            _ => Target::Method,
        }
    }

    fn declaration(&self, markers: &[CppAttribute], target: Target) -> Result<(), Failure> {
        for (index, marker) in markers.iter().enumerate() {
            if !self.is_tachyon_marker(marker)? {
                continue;
            }
            let neighbours: Vec<String> = markers
                .iter()
                .enumerate()
                .filter(|(other, _)| *other != index)
                .map(|(_, other)| other.name.clone())
                .collect();
            let placed = PlacedMarker {
                name: &marker.name,
                target,
                has_arguments: marker.has_arguments(),
                neighbours: &neighbours,
                class_markers: &self.class_markers,
            };
            if let Some((code, message)) = placed.problem(CppSource::EXTENSION, self.realm) {
                return Err(CppFailure::coded(code, self.path, marker.at, &message));
            }
        }
        Ok(())
    }

    fn is_tachyon_marker(&self, marker: &CppAttribute) -> Result<bool, Failure> {
        if let Some((namespace, plain)) = marker.name.rsplit_once("::") {
            return self.namespaced(marker, namespace, plain);
        }
        if Self::is_standard(&marker.name) {
            return Ok(false);
        }
        let Some(rule) = AnnotationRules::find(&marker.name) else {
            return Err(self.unknown(marker));
        };
        if rule.name != marker.name {
            return Err(self.refused(
                marker,
                &format!(
                    "C++ markers are PascalCase standard attributes: write [[{}]], not [[{}]]",
                    rule.name, marker.name
                ),
            ));
        }
        Ok(true)
    }

    fn namespaced(
        &self,
        marker: &CppAttribute,
        namespace: &str,
        plain: &str,
    ) -> Result<bool, Failure> {
        if !Self::OWN_NAMESPACES.contains(&namespace) {
            return Ok(false);
        }
        Err(self.refused(
            marker,
            &format!(
                "Tachyon markers have no namespace in C++: write [[{plain}]], not [[{}]]",
                marker.name
            ),
        ))
    }

    fn refused(&self, marker: &CppAttribute, message: &str) -> Failure {
        CppFailure::unreadable(self.path, marker.at, message)
    }

    fn unknown(&self, marker: &CppAttribute) -> Failure {
        let closest = AnnotationRules::rules()
            .filter(|rule| !rule.is_retired())
            .map(|rule| (Self::distance(rule.name, &marker.name), rule.name))
            .min();
        let message = match closest {
            Some((distance, name)) if distance <= Self::NEAR_MISS => format!(
                "[[{}]] is not a Tachyon annotation; did you mean [[{name}]]?",
                marker.name
            ),
            _ => format!(
                "[[{}]] is not a Tachyon annotation or a standard C++ attribute",
                marker.name
            ),
        };
        self.refused(marker, &message)
    }

    fn distance(left: &str, right: &str) -> usize {
        let right: Vec<char> = right.chars().collect();
        let mut previous: Vec<usize> = (0..=right.len()).collect();
        for (row, left_character) in left.chars().enumerate() {
            let mut current = vec![row + 1];
            for (column, right_character) in right.iter().enumerate() {
                let replaced = previous[column] + usize::from(left_character != *right_character);
                let best = replaced
                    .min(previous[column + 1] + 1)
                    .min(current[column] + 1);
                current.push(best);
            }
            previous = current;
        }
        previous[right.len()]
    }
}
