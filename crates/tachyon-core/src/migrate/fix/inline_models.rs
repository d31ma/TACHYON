use super::inline_model::InlineModel;
use crate::annotation_layout::InlineModelCheck;
use crate::model_layout::{ModelBlock, ModelBlocks};
use crate::route_types::{Model, ModelTable, Shape};
use std::path::Path;

pub(super) struct InlineModels<'a> {
    pub(super) relative: &'a str,
    pub(super) source: &'a str,
    pub(super) realm: &'a str,
    pub(super) models: Vec<InlineModel>,
}

impl<'a> InlineModels<'a> {
    const LANGUAGE: &'static str = "js";

    pub(super) fn read(relative: &'a str, source: &'a str) -> Option<Self> {
        let path = Path::new(relative);
        let blocks = ModelBlocks::read(path, source).ok()?;
        let models: Vec<InlineModel> = blocks
            .into_iter()
            .map(|block| Self::model(source, block))
            .collect();
        let file = Self {
            relative,
            source,
            realm: InlineModelCheck::realm(path),
            models,
        };
        file.has_models().then_some(file)
    }

    pub(super) fn block_text(&self, model: &InlineModel) -> &'a str {
        model.block.text(self.source)
    }

    pub(super) fn removal_end(&self, model: &InlineModel) -> usize {
        let end = model.block.end;
        let blank: usize = self.source[end..]
            .split_inclusive('\n')
            .take_while(|line| line.trim().is_empty())
            .map(str::len)
            .sum();
        end + blank
    }

    fn has_models(&self) -> bool {
        !self.models.is_empty()
    }

    fn model(source: &str, block: ModelBlock) -> InlineModel {
        let declared = ModelTable::declared_models(Self::LANGUAGE, block.text(source));
        let found = declared.into_iter().next().map(|(_, model)| model);
        InlineModel {
            block,
            model: found.unwrap_or(Model::Alias(Shape::Any)),
        }
    }
}
