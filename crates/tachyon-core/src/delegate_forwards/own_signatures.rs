use super::{DelegateBase, DelegateForwards, DelegateRefusal, MethodHeader, OwnMethods, Signature};
use crate::Failure;
use std::path::Path;

pub(super) struct OwnSignatures<'a> {
    language: &'a str,
    path: &'a Path,
    source: &'a str,
    masked: &'a str,
    base: &'a DelegateBase<'a>,
}

impl<'a> OwnSignatures<'a> {
    const OVERLOADS: [&'static str; 7] = ["java", "groovy", "kt", "cs", "fs", "hpp", "swift"];
    const LANGUAGE_NAMES: [(&'static str, &'static str); 3] =
        [("go", "Go"), ("rs", "Rust"), ("dart", "Dart")];
    const OVERLOAD_CODE: u16 = 1374;
    const OVERRIDE_CODE: u16 = 1375;

    pub(super) fn new(
        language: &'a str,
        path: &'a Path,
        (source, masked): (&'a str, &'a str),
        base: &'a DelegateBase<'a>,
    ) -> Self {
        Self {
            language,
            path,
            source,
            masked,
            base,
        }
    }

    pub(super) fn forwarded(&self, name: &str, header: &MethodHeader) -> Result<bool, Failure> {
        let own = self.own(name);
        let wanted = Signature::of(self.language, name, header);
        let signatures: Vec<(usize, Signature)> = own
            .iter()
            .map(|(at, own_header)| (*at, Signature::of(self.language, name, own_header)))
            .collect();
        let same = signatures
            .iter()
            .find(|(_, signature)| signature.parameters == wanted.parameters);
        let other_return = same.filter(|(_, signature)| signature.returns != wanted.returns);
        other_return.map_or(Ok(()), |(at, signature)| {
            Err(self.other_return(name, *at, (&signature.returns, &wanted.returns)))
        })?;
        let overloadable = Self::OVERLOADS.contains(&self.language);
        let refused = signatures
            .first()
            .filter(|_| [same.is_none(), overloadable] == [true, false]);
        refused.map_or(Ok(()), |(at, _)| Err(self.no_overload(name, *at)))?;
        Ok([own.is_empty(), same.is_none()] != [false, false])
    }

    fn own(&self, name: &str) -> Vec<(usize, MethodHeader)> {
        OwnMethods::starts(self.masked, name)
            .into_iter()
            .filter_map(|start| {
                let header =
                    MethodHeader::read(self.language, (self.source, self.masked), start, name)?;
                let at = start + self.masked[start..].find(name)?;
                Some((at, header))
            })
            .collect()
    }

    fn other_return(&self, name: &str, at: usize, (own, base): (&str, &str)) -> Failure {
        let delegate = DelegateForwards::class_name(self.path);
        let service = &self.base.service;
        let message = format!(
            "{delegate}.{name} has the same parameters as {service}.{name}, so it overrides it, but it returns {} instead of {}.",
            Self::shown(own),
            Self::shown(base),
        );
        let help = format!(
            "Return the type that {service}.{name} returns, or give this method another name (ADR 0112)."
        );
        self.refusal()
            .at(Self::OVERRIDE_CODE, (at, name.len()), (message, help))
    }

    fn no_overload(&self, name: &str, at: usize) -> Failure {
        let delegate = DelegateForwards::class_name(self.path);
        let service = &self.base.service;
        let field = &self.base.field;
        let language = Self::LANGUAGE_NAMES
            .iter()
            .find(|(extension, _)| *extension == self.language)
            .map_or(self.language, |(_, shown)| *shown);
        let message = format!(
            "{delegate}.{name} has other parameters than {service}.{name}. {language} has one method per name, so both cannot answer to {name}."
        );
        let help = format!(
            "Give this method another name, or give it the parameters of {service}.{name} and call the base through {field} inside it (ADR 0112)."
        );
        self.refusal()
            .at(Self::OVERLOAD_CODE, (at, name.len()), (message, help))
    }

    fn refusal(&self) -> DelegateRefusal<'a> {
        DelegateRefusal {
            path: self.path,
            source: self.source,
        }
    }

    fn shown(written: &str) -> &str {
        [written, "nothing"][usize::from(written.is_empty())]
    }
}
