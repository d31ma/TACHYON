#[Service]
#[derive(Default)]
struct AttachService {
    files: Files,
    name: String,
    digest: String,
    status: String,
    problem: String,
    selected: String,
}

impl AttachService {
    pub fn choose(&mut self) {
        self.problem = self.files.pick(".pdf,image/*", false).err().unwrap_or_default();
    }

    pub fn upload(&mut self) {
        let url = "https://uploads.example.com/inbox";
        let headers = "content-type: application/octet-stream";
        let sent = self.files.upload(&self.selected, "PUT", url, headers);
        self.problem = sent.err().unwrap_or_default();
    }

    #[OnEvent]
    pub fn picked(&mut self, picked: FilePicked) {
        self.name = format!("{}, {} bytes", picked.name, picked.size);
        self.digest = picked.sha256;
        self.selected = picked.id;
    }

    #[OnEvent]
    pub fn finished(&mut self, finished: UploadFinished) {
        self.status = format!("HTTP {} {}", finished.status, finished.error);
    }
}
