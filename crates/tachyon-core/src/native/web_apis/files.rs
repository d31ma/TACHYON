
#[derive(Clone, Copy, Debug, Default)]
pub struct Files;

impl Files {
    const API: &'static str = "files";

    pub fn pick(&self, accept: &str, multiple: bool) -> Result<(), String> {
        let arguments = format!("{},{multiple}", tac_quote(accept));
        TacWebHost::call(Self::API, "pick", &arguments)?;
        Ok(())
    }

    pub fn read(&self, id: &str, offset: u64, count: u32) -> Result<(), String> {
        let arguments = format!("{},{offset},{count}", tac_quote(id));
        TacWebHost::call(Self::API, "read", &arguments)?;
        Ok(())
    }

    pub fn release(&self, id: &str) -> Result<(), String> {
        TacWebHost::call(Self::API, "release", &tac_quote(id))?;
        Ok(())
    }

    pub fn upload(&self, id: &str, method: &str, url: &str, headers: &str) -> Result<String, String> {
        let arguments = [id, method, url, headers].map(tac_quote).join(",");
        let value = TacWebHost::call(Self::API, "upload", &arguments)?;
        Ok(value.as_text())
    }

    pub fn cancel(&self, upload: &str) -> Result<(), String> {
        TacWebHost::call(Self::API, "cancel", &tac_quote(upload))?;
        Ok(())
    }
}
