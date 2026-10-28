use super::java_code::JavaCode;
use super::java_constructor_provider::JavaConstructorProvider;
use super::java_declaration::JavaDeclaration;
use super::java_need_owner::JavaNeedOwner;
use crate::Failure;
use std::ops::Range;

pub(super) struct JavaDelegateBases;

impl JavaDelegateBases {
    const DELEGATE: &'static str = "@Delegate";
    const DELEGATE_WITH_ARGUMENTS: &'static str = "@Delegate(";
    const NEEDS: &'static str = "@Needs";
    const TYPE_ENDS: [u8; 2] = *b"};";

    pub(super) fn filled(
        source: &str,
        owner: &JavaNeedOwner<'_>,
    ) -> Result<(String, Vec<String>), Failure> {
        let code = JavaCode::new(source);
        let found = Self::base_fields(&code)
            .into_iter()
            .map(|(header, kind, name)| Self::provided(owner, header, (&kind, &name)))
            .collect::<Result<Vec<_>, Failure>>()?;
        let based: Vec<(Range<usize>, JavaConstructorProvider)> =
            found.into_iter().flatten().collect();
        let instances = based
            .iter()
            .filter_map(|(_, provider)| provider.needed())
            .map(|(instance, _)| instance.to_owned())
            .collect();
        Ok((Self::rewritten(source, &based), instances))
    }

    fn provided(
        owner: &JavaNeedOwner<'_>,
        header: Range<usize>,
        (kind, name): (&str, &str),
    ) -> Result<Option<(Range<usize>, JavaConstructorProvider)>, Failure> {
        Ok(owner
            .provider(kind, name)?
            .map(|provider| (header, provider)))
    }

    fn base_fields(code: &JavaCode) -> Vec<(Range<usize>, String, String)> {
        code.top_level_types()
            .iter()
            .filter(|declared| Self::is_delegate(code, declared.keyword_at))
            .filter_map(|declared| code.body_after(declared.name_end))
            .flat_map(|body| code.member_headers(&body))
            .filter_map(|header| Self::needed(code, header))
            .collect()
    }

    fn needed(code: &JavaCode, header: Range<usize>) -> Option<(Range<usize>, String, String)> {
        let (kind, name) = JavaDeclaration::parse(&code.text(&header)).needed_field()?;
        Some((header, kind, name))
    }

    fn is_delegate(code: &JavaCode, keyword_at: usize) -> bool {
        let before = &code.bytes()[..keyword_at];
        let start = before
            .iter()
            .rposition(|byte| Self::TYPE_ENDS.contains(byte))
            .map_or(0, |at| at + 1);
        String::from_utf8_lossy(&before[start..])
            .split_whitespace()
            .any(|word| {
                [
                    word == Self::DELEGATE,
                    word.starts_with(Self::DELEGATE_WITH_ARGUMENTS),
                ]
                .contains(&true)
            })
    }

    fn rewritten(source: &str, based: &[(Range<usize>, JavaConstructorProvider)]) -> String {
        based
            .iter()
            .rev()
            .fold(source.to_owned(), |text, (header, provider)| {
                Self::filled_field(text, header, provider)
            })
    }

    fn filled_field(
        mut text: String,
        header: &Range<usize>,
        provider: &JavaConstructorProvider,
    ) -> String {
        text.insert_str(header.end, &format!(" = {}", provider.expression()));
        let marker = text[header.clone()]
            .find(Self::NEEDS)
            .map(|at| header.start + at);
        let blanked = marker.map_or(0..0, |at| at..at + Self::NEEDS.len());
        text.replace_range(blanked.clone(), &" ".repeat(blanked.len()));
        text
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::super::java_service_needs::JavaServiceNeeds;
    use super::JavaDelegateBases;
    use std::path::{Path, PathBuf};

    struct BaseFixture;

    impl BaseFixture {
        const TAX: &'static str = "@Service\nclass TaxService {\n    int asked = 0;\n}\n";
        const QUOTE: &'static str = "@Service\nclass QuoteService {\n    @Needs\n    private final TaxDelegate taxDelegate = new TaxDelegate();\n}\n\n\
             @Delegate\nclass TaxDelegate {\n    @Needs\n    private final TaxService taxService;\n\n    int rate(String region) {\n        return taxService.rate(region) * 2;\n    }\n}\n";

        fn filled(tax: &str) -> Result<(String, Vec<String>), crate::Failure> {
            let services = [
                (
                    String::from("tax"),
                    String::from("TaxService"),
                    PathBuf::from("client/services/TaxService.java"),
                ),
                (
                    String::from("quote"),
                    String::from("QuoteService"),
                    PathBuf::from("client/services/QuoteService.java"),
                ),
            ];
            let texts = [String::from(tax), String::from(Self::QUOTE)];
            let needs = JavaServiceNeeds::read(&services, &texts).expect("page");
            let path = Path::new("client/services/QuoteService.java");
            let owner = needs
                .owner("quote", path, (Self::QUOTE, Self::QUOTE, "QuoteService"))
                .expect("owner");
            JavaDelegateBases::filled(Self::QUOTE, &owner)
        }
    }

    #[test]
    fn a_delegate_base_field_takes_the_page_service() {
        let (text, instances) = BaseFixture::filled(BaseFixture::TAX).expect("filled");
        assert_eq!(instances, [String::from("tax")]);
        assert!(
            text.contains(
                "private final TaxService taxService = (TacService0.TaxService) TacNeeds.need(\"tax\");"
            ),
            "{text}"
        );
        assert!(
            text.contains("private final TaxDelegate taxDelegate = new TaxDelegate();"),
            "{text}"
        );
        assert_eq!(text.matches("@Needs").count(), 1, "{text}");
    }

    #[test]
    fn a_delegate_base_with_one_instance_per_component_is_refused() {
        let tax = format!(
            "@Lifetime({{\"value\":\"prototype\"}})\n{}",
            BaseFixture::TAX
        );
        let failure = BaseFixture::filled(&tax).expect_err("prototype base");
        let message = failure.to_string();
        assert!(message.contains("TY1406"), "{message}");
        assert!(
            message.contains("one instance for each component"),
            "{message}"
        );
    }
}
