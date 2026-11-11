
#[derive(Clone, Copy, Debug, Default)]
pub struct StorageSpace;

impl StorageSpace {
    const API: &'static str = "storageSpace";

    pub fn estimate(&self) -> Result<(), String> {
        TacWebHost::call(Self::API, "estimate", "")?;
        Ok(())
    }

    pub fn persist(&self) -> Result<(), String> {
        TacWebHost::call(Self::API, "persist", "")?;
        Ok(())
    }
}
