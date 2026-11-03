#[derive(Clone, Debug)]
pub struct FilePickerResult {
    pub status: FilePickerStatus,
    pub handle: Option<String>,
    pub name: Option<String>,
    pub text: Option<String>,
}

impl FilePickerResult {
    fn read(answer: &str) -> Self {
        let values = TacScanner::new(answer).object();
        let status = tac_entry_text(&values, "status")
            .map_or(FilePickerStatus::Unavailable, |value| {
                FilePickerStatus::read(value)
            });
        Self {
            status,
            handle: tac_entry_text(&values, "handle").cloned(),
            name: tac_entry_text(&values, "name").cloned(),
            text: tac_entry_text(&values, "text").cloned(),
        }
    }
}
