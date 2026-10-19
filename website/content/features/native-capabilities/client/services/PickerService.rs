#[Service]
#[derive(Default)]
struct PickerService {
    file_picker: FilePicker,
    status: String,
}

impl PickerService {
    pub fn choose(&mut self) {
        let options = FilePickerOptions { multiple: false, types: vec![String::from("text/plain")] };
        let chosen = self.file_picker.open(&options);
        self.status = chosen.status.as_str().to_owned();
        self.read(chosen.handle);
    }

    fn read(&mut self, handle: Option<String>) {
        if let Some(handle) = handle {
            self.status = self.file_picker.read_text(&handle).status.as_str().to_owned();
            self.file_picker.release(&handle);
        }
    }
}
