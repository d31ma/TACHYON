#[derive(Clone, Debug, Default)]
pub struct FilePickerOptions {
    pub multiple: bool,
    pub types: Vec<String>,
}
