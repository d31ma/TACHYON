
#[derive(Clone, Copy, Debug, Default)]
pub struct Crypto;

impl Crypto {
    const API: &'static str = "crypto";
    const NO_ARGUMENTS: &'static str = "";

    pub fn supported(&self) -> Result<bool, String> {
        Ok(TacWebHost::call(Self::API, "supported", Self::NO_ARGUMENTS)?.as_flag())
    }

    pub fn check(&self) -> Result<String, String> {
        Self::request("check", String::new())
    }

    pub fn make_key_pair(&self, key: &str, algorithm: &str, keep: bool) -> Result<String, String> {
        Self::request("makeKeyPair", format!("{},{keep}", Self::quoted(&[key, algorithm])))
    }

    pub fn public_key(&self, key: &str) -> Result<String, String> {
        Self::request("publicKey", Self::quoted(&[key]))
    }

    pub fn sign(&self, key: &str, data: &str) -> Result<String, String> {
        Self::request("sign", Self::quoted(&[key, data]))
    }

    pub fn verify(&self, public_key: &str, data: &str, signature: &str) -> Result<String, String> {
        Self::request("verify", Self::quoted(&[public_key, data, signature]))
    }

    pub fn agree(&self, key: &str, peer_public_key: &str, salt: &str, info: &str, new_key: &str, key_use: &str, keep: bool) -> Result<String, String> {
        let texts = Self::quoted(&[key, peer_public_key, salt, info, new_key, key_use]);
        Self::request("agree", format!("{texts},{keep}"))
    }

    pub fn derive_key(&self, from_key: &str, salt: &str, info: &str, new_key: &str, key_use: &str, keep: bool) -> Result<String, String> {
        let texts = Self::quoted(&[from_key, salt, info, new_key, key_use]);
        Self::request("deriveKey", format!("{texts},{keep}"))
    }

    pub fn import_key(&self, new_key: &str, secret: &str, key_use: &str, keep: bool) -> Result<String, String> {
        Self::request("importKey", format!("{},{keep}", Self::quoted(&[new_key, secret, key_use])))
    }

    pub fn from_code(&self, new_key: &str, code: &str, salt: &str, key_use: &str, keep: bool) -> Result<String, String> {
        Self::request("fromCode", format!("{},{keep}", Self::quoted(&[new_key, code, salt, key_use])))
    }

    pub fn encrypt(&self, key: &str, data: &str, aad: &str) -> Result<String, String> {
        Self::request("encrypt", Self::quoted(&[key, data, aad]))
    }

    pub fn decrypt(&self, key: &str, sealed: &str, aad: &str) -> Result<String, String> {
        Self::request("decrypt", Self::quoted(&[key, sealed, aad]))
    }

    pub fn hkdf(&self, input: &str, salt: &str, info: &str, length: usize) -> Result<String, String> {
        Self::request("hkdf", format!("{},{length}", Self::quoted(&[input, salt, info])))
    }

    pub fn hash(&self, data: &str) -> Result<String, String> {
        Self::request("hash", Self::quoted(&[data]))
    }

    pub fn forget(&self, key: &str) -> Result<String, String> {
        Self::request("forget", Self::quoted(&[key]))
    }

    pub fn forget_all(&self) -> Result<String, String> {
        Self::request("forgetAll", String::new())
    }

    fn quoted(values: &[&str]) -> String {
        values.iter().map(|value| tac_quote(value)).collect::<Vec<_>>().join(",")
    }

    fn request(call: &str, arguments: String) -> Result<String, String> {
        Ok(TacWebHost::call(Self::API, call, &arguments)?.as_text())
    }
}
