#[Service]
#[derive(Default)]
struct PickerService {
    file_picker: FilePicker,
    status: String,
}

impl PickerService {
    fn choose(&mut self) {
        let options = FilePickerOptions { multiple: false, types: vec![String::from("text/plain")] };
        let chosen = self.file_picker.open(&options);
        self.status = chosen.status.as_str().to_owned();
        if let Some(handle) = chosen.handle {
            self.status = self.file_picker.read_text(&handle).status.as_str().to_owned();
            self.file_picker.release(&handle);
        }
    }
}
