use super::java::{refused_enum, refused_field, refused_record};
use super::piece::{Body, Piece};

pub(super) struct GroovyModel;

impl GroovyModel {
    const TYPE_MODIFIERS: [&'static str; 3] = ["public", "final", "abstract"];

    pub(super) fn first_refused(code: Piece<'_>) -> Option<usize> {
        code.first_framework_annotation()
            .or_else(|| Self::first_refused_item(code))
    }

    fn first_refused_item(code: Piece<'_>) -> Option<usize> {
        code.statements().into_iter().find_map(|item| {
            let declaration = item.after_annotations().after_words(&Self::TYPE_MODIFIERS);
            Self::refused_declaration(declaration)
        })
    }

    fn refused_declaration(declaration: Piece<'_>) -> Option<usize> {
        let kind = declaration.first_word();
        let rest = declaration.from(kind.len()).trim();
        match kind {
            "import" => None,
            "record" => refused_record(rest),
            "enum" => refused_enum(rest),
            "class" => Self::refused_class(rest),
            _ => Some(declaration.at),
        }
    }

    fn refused_class(declaration: Piece<'_>) -> Option<usize> {
        declaration
            .body()
            .map_or(Some(declaration.at), |body| Self::refused_body(&body))
    }

    fn refused_body(body: &Body<'_>) -> Option<usize> {
        let end = (!body.has_plain_end()).then_some(body.rest.at);
        end.or_else(|| Self::first_refused_member(body.inner))
    }

    fn first_refused_member(inner: Piece<'_>) -> Option<usize> {
        inner.statements().into_iter().find_map(refused_field)
    }
}
