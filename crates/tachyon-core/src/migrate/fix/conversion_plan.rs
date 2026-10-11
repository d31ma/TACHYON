use super::ManualStep;
use super::edits::TextEdit;

#[derive(Default)]
pub(super) struct ConversionPlan {
    pub(super) edits: Vec<TextEdit>,
    pub(super) notes: Vec<String>,
    pub(super) steps: Vec<ManualStep>,
}
