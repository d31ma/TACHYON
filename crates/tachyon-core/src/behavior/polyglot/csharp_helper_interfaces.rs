use super::super::Input;
use super::Type;
use crate::Failure;

pub(super) struct CsharpHelperInterfaces;

impl CsharpHelperInterfaces {
    const ADAPTERS: [&'static str; 3] = [
        "BehaviorHTTPAdapter",
        "BehaviorCacheAdapter",
        "BehaviorEntityAdapter",
    ];

    pub(super) fn allowed(input: &Input<'_>, language: &str, ty: &Type) -> Result<bool, Failure> {
        Self::selected(input.path, language, ty)
            .then(|| Self::implemented(input, ty))
            .transpose()
            .map(|allowed| allowed.unwrap_or(false))
    }

    fn selected(path: &std::path::Path, language: &str, ty: &Type) -> bool {
        matches!(
            (
                language,
                crate::annotation_rules::Realm::of(path),
                ty.policies.contains_key("helper")
            ),
            ("cs", Some(crate::annotation_rules::Realm::Tac), true)
        )
    }

    fn implemented(input: &Input<'_>, ty: &Type) -> Result<bool, Failure> {
        let declarations = super::declarations(input, "cs")?;
        let interfaces = Self::interfaces(input, &declarations);
        Ok(Self::bases(input, ty)
            .iter()
            .all(|name| Self::known(name, &interfaces)))
    }

    fn interfaces<'a>(input: &Input<'_>, types: &'a [Type]) -> Vec<&'a str> {
        types
            .iter()
            .filter(|ty| &input.code[ty.keyword.clone()] == "interface")
            .map(|ty| ty.name.as_str())
            .collect()
    }

    fn bases<'a>(input: &'a Input<'_>, ty: &Type) -> Vec<&'a str> {
        input.code[ty.declaration..ty.body.start]
            .split_once(':')
            .map_or_else(Vec::new, |(_, bases)| {
                bases
                    .trim_end_matches('{')
                    .split(',')
                    .map(str::trim)
                    .collect()
            })
    }

    fn known(name: &str, interfaces: &[&str]) -> bool {
        Self::ADAPTERS.contains(&name) || interfaces.contains(&name)
    }
}
