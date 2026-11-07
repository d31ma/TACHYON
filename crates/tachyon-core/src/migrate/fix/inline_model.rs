use crate::model_layout::ModelBlock;
use crate::route_types::Model;

pub(super) struct InlineModel {
    pub(super) block: ModelBlock,
    pub(super) model: Model,
}
