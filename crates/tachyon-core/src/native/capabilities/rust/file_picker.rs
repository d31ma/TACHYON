#[derive(Clone, Debug)]
pub struct FilePicker {
    scope: std::rc::Rc<NativeCapabilityScope>,
}

impl Default for FilePicker {
    fn default() -> Self {
        Self {
            scope: NativeCapabilities::scope(),
        }
    }
}

impl FilePicker {
    pub fn available(&self) -> bool {
        FilePickerResult::read(&self.scope.call("filePicker.available", Vec::new())).status
            == FilePickerStatus::Available
    }

    pub fn open(&self, options: &FilePickerOptions) -> FilePickerResult {
        Self::request_options(options).map_or_else(
            |failure| failure,
            |values| FilePickerResult::read(&self.scope.call("filePicker.open", values)),
        )
    }

    fn request_options(
        options: &FilePickerOptions,
    ) -> Result<Vec<(String, TacValue)>, FilePickerResult> {
        Self::valid_options(options)
            .then(|| {
                let types = options.types.iter().cloned().map(TacValue::Text).collect();
                vec![(String::from("types"), TacValue::List(types))]
            })
            .ok_or_else(|| FilePickerResult::read("{\"status\":\"invalid\"}"))
    }

    fn valid_options(options: &FilePickerOptions) -> bool {
        !options.multiple
            && options.types.len() <= 2
            && options
                .types
                .iter()
                .all(|value| ["text/plain", "application/json"].contains(&value.as_str()))
    }

    pub fn read_text(&self, handle: &str) -> FilePickerResult {
        self.handle("readText", handle)
    }
    pub fn release(&self, handle: &str) -> FilePickerResult {
        self.handle("release", handle)
    }

    fn handle(&self, operation: &str, handle: &str) -> FilePickerResult {
        FilePickerResult::read(&self.scope.call(
            &format!("filePicker.{operation}"),
            vec![(String::from("handle"), TacValue::Text(handle.to_owned()))],
        ))
    }
}
