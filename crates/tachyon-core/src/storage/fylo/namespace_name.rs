use super::collection_name::CollectionName;
use crate::Failure;
use crate::storage::failure;

pub(super) struct NamespaceName;

impl NamespaceName {
    const DEVICE_NAMES: [&'static str; 22] = [
        "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
        "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
    ];

    pub(super) fn checked(name: &str) -> Result<&str, Failure> {
        let allowed = Self::is_allowed(name);
        allowed.then_some(name).ok_or_else(Self::refused)
    }

    fn is_allowed(name: &str) -> bool {
        let checks = [
            CollectionName::checked(name).is_ok(),
            !Self::DEVICE_NAMES.contains(&name),
        ];
        checks == [true, true]
    }

    fn refused() -> Failure {
        failure(
            "A FYLO namespace has 2 to 64 lowercase letters, digits and hyphens, starts and ends with a letter or digit, and is not a reserved word or a Windows device name.",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::NamespaceName;

    #[test]
    fn plain_lowercase_names_are_namespaces() {
        for name in ["public", "slice-12", "a1", "tenant-example-com"] {
            assert!(NamespaceName::checked(name).is_ok(), "{name}");
        }
    }

    #[test]
    fn unsafe_reserved_and_device_names_are_refused() {
        for name in [
            "", "a", "-x", "x-", "Slice", "a_b", "a/b", "..", "sql", "queue", "con", "lpt9",
        ] {
            assert!(NamespaceName::checked(name).is_err(), "{name}");
        }
    }
}
