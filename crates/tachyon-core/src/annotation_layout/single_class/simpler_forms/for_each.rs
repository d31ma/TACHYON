#![allow(clippy::needless_bitwise_bool)]

use super::code_language::CodeLanguage;
use super::form_pattern::FormPattern;
use super::if_part::IfPart;
use super::loop_shapes::LoopShapes;
use super::pieces::Pieces;
use super::shape::Shape;
use super::simpler_form::SimplerForm;

pub(super) struct ForEach;

impl ForEach {
    const LEAKING: [CodeLanguage; 2] = [CodeLanguage::Python, CodeLanguage::Php];

    pub(super) fn find(pieces: &Pieces<'_>) -> Vec<SimplerForm> {
        let shapes = LoopShapes::of(pieces.language());
        (0..pieces.len())
            .filter(|&at| pieces.line_start(at))
            .filter_map(|at| Self::form(pieces, &shapes, at))
            .collect()
    }

    fn form(pieces: &Pieces<'_>, shapes: &LoopShapes, at: usize) -> Option<SimplerForm> {
        let header = shapes
            .headers
            .iter()
            .find_map(|template| Shape::read(pieces, at, template))?;
        let body = Self::body(pieces, &header)?;
        let item = Self::item(pieces, shapes, &body, &header)?;
        let item_end = pieces.line_after(item.start);
        let index = header.get("I");
        let items = item.get("XS");
        let bounded = shapes.bounds(header.get("BOUND"), items) | (header.get("XS") == items);
        let last_word = items.rsplit('.').next().unwrap_or(items);
        let reused = (item_end..body.to).any(|piece| pieces.is_any(piece, &[index, last_word]));
        let leaks = pieces.language().is_one_of(&Self::LEAKING)
            & (body.end..pieces.len()).any(|piece| pieces.is(piece, index));
        let declaration = shapes.allows(item.get("D"));
        let fits = bounded
            & !items.is_empty()
            & !reused
            & !leaks
            & declaration
            & pieces.is_whole_lines(at, header.end)
            & pieces.is_whole_lines(item.start, item_end)
            & (item.start == body.from);
        let text = item.fill(shapes.output);
        fits.then(|| {
            SimplerForm::new(
                FormPattern::ForEach,
                pieces.start(at),
                pieces.line_tail(item_end - 1),
                format!("{text}\n"),
            )
        })
    }

    fn body(pieces: &Pieces<'_>, header: &Shape) -> Option<IfPart> {
        let last = header.end - 1;
        let column = pieces.column(header.start);
        match pieces.language() {
            CodeLanguage::Python => IfPart::indented(pieces, last, column),
            CodeLanguage::Ruby => IfPart::ended(pieces, header.end, column),
            _ => IfPart::after_head(pieces, last).filter(IfPart::is_braced),
        }
    }

    fn item(
        pieces: &Pieces<'_>,
        shapes: &LoopShapes,
        body: &IfPart,
        header: &Shape,
    ) -> Option<Shape> {
        let line_end = pieces.line_after(body.from);
        let equals = pieces.find_top(body.from, line_end, &["=", ":="])?;
        let name = equals - 1;
        let declared = pieces.slice(body.from, name);
        let mut item = shapes
            .items
            .iter()
            .find_map(|shape| Shape::read(pieces, name, &format!("@X {shape}")))?;
        let ending = item.end + usize::from(pieces.is(item.end, ";"));
        let same = item.get("I") == header.get("I");
        let items = LoopShapes::items_of(item.get("XS"), item.get("GETTER"));
        item.start = body.from;
        item.end = ending;
        let alone = ending == line_end;
        (same & alone).then(|| item.with("D", declared).with("XS", &items))
    }
}
