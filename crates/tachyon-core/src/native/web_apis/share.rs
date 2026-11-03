
#[derive(Clone, Copy, Debug, Default)]
pub struct Share;

impl Share {
    pub fn open(&self, title: &str, text: &str, url: &str) -> Result<(), String> {
        let arguments = [tac_quote(title), tac_quote(text), tac_quote(url)].join(",");
        TacWebHost::call("share", "open", &arguments)?;
        Ok(())
    }
}
