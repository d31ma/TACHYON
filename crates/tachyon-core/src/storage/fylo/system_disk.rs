use serde_json::Value;

use crate::Failure;
use crate::storage::failure;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SystemDisk {
    Refused = 0,
    Allowed = 1,
}

impl SystemDisk {
    pub(super) const KEY: &'static str = "allowsystemdisk";
    const OPTIONS: [&'static [&'static str]; 2] = [&[], &["--allow-system-disk"]];

    pub(super) fn from_settings(settings: &Value) -> Result<Self, Failure> {
        let allowed = settings.get(Self::KEY).map_or(Some(false), Value::as_bool);
        Self::chosen(allowed)
    }

    pub(super) fn options(self) -> &'static [&'static str] {
        Self::OPTIONS[self as usize]
    }

    fn chosen(allowed: Option<bool>) -> Result<Self, Failure> {
        match allowed {
            Some(false) => Ok(Self::Refused),
            Some(true) => Ok(Self::Allowed),
            None => Err(failure(
                "The FYLO datasource setting \"allowsystemdisk\" is true or false.",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::SystemDisk;

    #[test]
    fn a_datasource_without_the_setting_keeps_the_system_disk_refused() {
        let disk = SystemDisk::from_settings(&json!({"driver": "fylo"})).ok();
        assert_eq!(disk, Some(SystemDisk::Refused));
        assert!(SystemDisk::Refused.options().is_empty());
    }

    #[test]
    fn the_setting_passes_the_fylo_opt_out() {
        let disk = SystemDisk::from_settings(&json!({"allowsystemdisk": true})).ok();
        assert_eq!(disk, Some(SystemDisk::Allowed));
        assert_eq!(SystemDisk::Allowed.options(), ["--allow-system-disk"]);
    }

    #[test]
    fn the_refusal_names_the_setting_and_the_fylo_variable() {
        let message = crate::storage::fylo::FyloError::coded("ESYSTEMDISK")
            .into_failure()
            .to_string();
        assert!(message.contains("ESYSTEMDISK"), "{message}");
        assert!(message.contains("\"allowsystemdisk\": true"), "{message}");
        assert!(message.contains("FYLO_ALLOW_SYSTEM_DISK=1"), "{message}");
    }

    #[test]
    fn a_setting_that_is_not_true_or_false_is_refused() {
        assert!(SystemDisk::from_settings(&json!({"allowsystemdisk": "yes"})).is_err());
    }
}
