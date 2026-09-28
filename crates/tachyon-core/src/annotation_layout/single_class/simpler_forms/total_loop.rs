#![allow(clippy::needless_bitwise_bool)]

use super::form_pattern::FormPattern;
use super::pieces::Pieces;
use super::shape::Shape;
use super::simpler_form::SimplerForm;
use super::total_shape::TotalShape;

pub(super) struct TotalLoop;

impl TotalLoop {
    const EMPTY_TEXTS: [&'static str; 2] = ["\"\"", "''"];
    const ZERO: &'static str = "0";

    pub(super) fn find(pieces: &Pieces<'_>) -> Vec<SimplerForm> {
        let language = pieces.language();
        TotalShape::SHAPES
            .iter()
            .filter(|shape| shape.language == language)
            .flat_map(|shape| {
                (0..pieces.len())
                    .filter(|&at| pieces.line_start(at))
                    .filter_map(move |at| Self::form(pieces, shape, at))
            })
            .collect()
    }

    fn form(pieces: &Pieces<'_>, shape: &TotalShape, at: usize) -> Option<SimplerForm> {
        let start = Shape::read(pieces, at, shape.start)?;
        let initial = start.get("Z");
        let joins = Self::EMPTY_TEXTS.contains(&initial);
        let known = joins | (initial == Self::ZERO);
        let output = Some(shape.output(joins)).filter(|text| known & !text.is_empty())?;
        let header = Shape::read(pieces, start.end, shape.header)?;
        let body = Shape::read(pieces, header.end, shape.body(joins))?;
        let braced = pieces.language().is_braced();
        let closing = header.end - 1;
        let close = pieces.partner(closing).unwrap_or(body.end);
        let block_end = [pieces.line_after(body.start), close][usize::from(braced)];
        let ends_block = [
            Self::ends_indented(pieces, &header, body.end),
            close == body.end,
        ][usize::from(braced)];
        let same = (start.get("T") == body.get("T")) & (header.get("V") == body.get("V"));
        let separate = !header
            .get("XS")
            .split('.')
            .any(|word| word == start.get("T"));
        let lines = pieces.line_start(start.end)
            & pieces.line_start(header.end)
            & pieces.line_start(body.end)
            & pieces.is_plain(at, block_end);
        let items = header.get("XS");
        let text = start.fill(output).replace("{XS}", items);
        let end = [pieces.line_tail(body.end - 1), pieces.end(close)][usize::from(braced)];
        let ending = ["\n", ""][usize::from(braced)];
        let fits = same & separate & lines & ends_block;
        fits.then(|| {
            SimplerForm::new(
                FormPattern::TotalLoop,
                pieces.start(at),
                end,
                format!("{text}{ending}"),
            )
        })
    }

    fn ends_indented(pieces: &Pieces<'_>, header: &Shape, after: usize) -> bool {
        pieces.column(after) <= pieces.column(header.start)
    }
}
