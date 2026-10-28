use super::{
    ArgumentRange, DelegateBase, DelegateForwards, DelegateRefusal, Overload, OwnMethods,
    SourceLines,
};
use crate::Failure;
use std::path::Path;

pub(super) struct DynamicDispatch<'a> {
    language: &'a str,
    path: &'a Path,
    source: &'a str,
    masked: &'a str,
    base: &'a DelegateBase<'a>,
    service_masked: String,
}

impl<'a> DynamicDispatch<'a> {
    const LANGUAGES: [&'static str; 6] = ["js", "ts", "py", "php", "rb", "lua"];
    const CODE: u16 = 1374;

    pub(super) fn overloads(
        path: &'a Path,
        (source, masked): (&'a str, &'a str),
        base: &'a DelegateBase<'a>,
        methods: &[(String, usize)],
    ) -> Result<Vec<Overload>, Failure> {
        let language = path
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default();
        let dynamic = Self::LANGUAGES.contains(&language);
        let service_masked = dynamic
            .then(|| {
                crate::annotation_layout::masked_source(base.service_path, base.service_source).ok()
            })
            .flatten()
            .unwrap_or_default();
        let dispatch = Self {
            language,
            path,
            source,
            masked,
            base,
            service_masked,
        };
        let lines = SourceLines::with_offsets(&dispatch.service_masked);
        let compared: Vec<Option<Overload>> = methods
            .iter()
            .filter_map(|(name, line)| {
                let (at, text) = lines.get(*line)?;
                Some((name, at + SourceLines::indentation(text).len()))
            })
            .map(|(name, base_start)| dispatch.compared(name, base_start))
            .collect::<Result<_, Failure>>()?;
        Ok(compared.into_iter().flatten().collect())
    }

    fn compared(&self, name: &str, base_start: usize) -> Result<Option<Overload>, Failure> {
        let own = OwnMethods::starts(self.masked, name)
            .first()
            .and_then(|start| {
                let range =
                    ArgumentRange::read(self.language, (self.source, self.masked), *start, name)?;
                Some((self.name_at(*start, name), range))
            });
        let base = ArgumentRange::read(
            self.language,
            (self.base.service_source, &self.service_masked),
            base_start,
            name,
        );
        let pair = own.zip(base);
        let clash =
            pair.filter(|((_, own), base)| [own != base, own.overlaps(*base)] == [true, true]);
        clash.map_or(Ok(()), |((at, own), base)| {
            Err(self.overlap(name, at, (own, base)))
        })?;
        Ok(pair
            .filter(|((_, own), base)| !own.overlaps(*base))
            .map(|((renamed_at, own), _)| Overload {
                name: name.to_owned(),
                renamed_at,
                own,
            }))
    }

    fn name_at(&self, start: usize, name: &str) -> usize {
        let (_, line_end) = SourceLines::line_around(self.masked, start);
        let offset = ArgumentRange::name_offset(&self.masked[start..line_end], name);
        start + offset.unwrap_or_default()
    }

    fn overlap(
        &self,
        name: &str,
        at: usize,
        (own, base): (ArgumentRange, ArgumentRange),
    ) -> Failure {
        let delegate = DelegateForwards::class_name(self.path);
        let service = &self.base.service;
        let message = format!(
            "{delegate}.{name} takes {} arguments and {service}.{name} takes {}, so a call could match both.",
            own.shown(),
            base.shown(),
        );
        let help = format!(
            "Give {delegate}.{name} the same parameters as {service}.{name} to override it, or argument counts that {service}.{name} never takes to overload it (ADR 0112)."
        );
        DelegateRefusal {
            path: self.path,
            source: self.source,
        }
        .at(Self::CODE, (at, name.len()), (message, help))
    }
}
