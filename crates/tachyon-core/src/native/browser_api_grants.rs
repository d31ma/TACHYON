use super::config::config_failure;
use crate::Failure;

pub(crate) struct BrowserApiGrants;

impl BrowserApiGrants {
    const KNOWN: [&'static str; 4] = ["clipboard", "geolocation", "notifications", "share"];
    const META_NAME: &'static str = "tachyon-browser-apis";

    pub(crate) fn checked(grants: Vec<String>) -> Result<Vec<String>, Failure> {
        Self::ensure_known(&grants)?;
        Self::ensure_unique(&grants)?;
        Ok(grants)
    }

    pub(crate) fn meta_tag(grants: &[String]) -> String {
        format!(
            "<meta name=\"{}\" content=\"{}\">",
            Self::META_NAME,
            grants.join(" ")
        )
    }

    fn ensure_known(grants: &[String]) -> Result<(), Failure> {
        match Self::unknown(grants) {
            Some(grant) => Err(config_failure(&format!(
                "'{grant}' in tac.browserApis is not a browser API that needs a grant. Grant any of: {}.",
                Self::KNOWN.join(", ")
            ))),
            None => Ok(()),
        }
    }

    fn unknown(grants: &[String]) -> Option<&String> {
        grants
            .iter()
            .find(|grant| !Self::KNOWN.contains(&grant.as_str()))
    }

    fn ensure_unique(grants: &[String]) -> Result<(), Failure> {
        let unique: std::collections::BTreeSet<&String> = grants.iter().collect();
        if unique.len() == grants.len() {
            return Ok(());
        }
        Err(config_failure(
            "Manifest tac.browserApis lists a browser API more than once.",
        ))
    }
}
