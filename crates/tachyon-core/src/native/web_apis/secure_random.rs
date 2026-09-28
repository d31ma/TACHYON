
#[derive(Clone, Copy, Debug, Default)]
pub struct SecureRandom;

impl SecureRandom {
    pub fn bytes(&self, count: usize) -> Result<Vec<u8>, String> {
        let value = TacWebHost::call("secureRandom", "bytes", &count.to_string())?;
        Ok(Self::byte_values(Self::items(&value)))
    }

    pub fn uuid(&self) -> Result<String, String> {
        let value = TacWebHost::call("secureRandom", "uuid", "")?;
        Ok(value.as_text())
    }

    fn items(value: &TacValue) -> &[TacValue] {
        match value {
            TacValue::List(items) => items,
            _ => &[],
        }
    }

    fn byte_values(items: &[TacValue]) -> Vec<u8> {
        items
            .iter()
            .map(|item| u8::try_from(item.as_int()).unwrap_or_default())
            .collect()
    }
}
