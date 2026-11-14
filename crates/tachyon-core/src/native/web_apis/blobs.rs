
#[derive(Clone, Copy, Debug, Default)]
pub struct Blobs;

impl Blobs {
    const API: &'static str = "blobs";

    pub fn save_held(&self, file_id: &str, key: &str) -> Result<(), String> {
        Self::send("saveHeld", &[file_id, key])
    }

    pub fn download(&self, url: &str, key: &str, headers: &str) -> Result<(), String> {
        Self::send("download", &[url, key, headers])
    }

    pub fn list(&self, prefix: &str) -> Result<(), String> {
        Self::send("list", &[prefix])
    }

    pub fn read(&self, key: &str, offset: u64, count: u32) -> Result<(), String> {
        let arguments = format!("{},{offset},{count}", tac_quote(key));
        TacWebHost::call(Self::API, "read", &arguments)?;
        Ok(())
    }

    pub fn remove(&self, key: &str) -> Result<(), String> {
        Self::send("remove", &[key])
    }

    pub fn open(&self, key: &str) -> Result<(), String> {
        Self::send("open", &[key])
    }

    pub fn url(&self, key: &str) -> Result<String, String> {
        Ok(Self::ask("url", &[key])?.as_text())
    }

    pub fn close(&self, key: &str) -> Result<(), String> {
        Self::send("close", &[key])
    }

    pub fn upload(&self, key: &str, method: &str, url: &str, headers: &str) -> Result<String, String> {
        Ok(Self::ask("upload", &[key, method, url, headers])?.as_text())
    }

    pub fn cancel(&self, upload: &str) -> Result<(), String> {
        Self::send("cancel", &[upload])
    }

    fn send(call: &str, texts: &[&str]) -> Result<(), String> {
        Self::ask(call, texts)?;
        Ok(())
    }

    fn ask(call: &str, texts: &[&str]) -> Result<TacValue, String> {
        let arguments: Vec<String> = texts.iter().map(|text| tac_quote(text)).collect();
        TacWebHost::call(Self::API, call, &arguments.join(","))
    }
}
