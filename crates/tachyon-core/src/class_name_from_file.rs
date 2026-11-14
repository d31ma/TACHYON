use std::path::Path;

pub(crate) struct ClassNameFromFile;

impl ClassNameFromFile {
    const NAME_BREAKS: [char; 2] = ['_', '-'];

    pub(crate) fn class_of(path: &Path) -> String {
        let stem = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default();
        stem.split(Self::NAME_BREAKS)
            .map(Self::capitalized)
            .collect()
    }

    fn capitalized(part: &str) -> String {
        let mut characters = part.chars();
        let first = characters.next().map(char::to_uppercase);
        first.map(String::from_iter).unwrap_or_default() + characters.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::ClassNameFromFile;
    use std::path::Path;

    #[test]
    fn a_snake_case_file_names_its_class_in_pascal_case() {
        assert_eq!(
            ClassNameFromFile::class_of(Path::new("client/services/order_service.py")),
            "OrderService"
        );
    }

    #[test]
    fn a_pascal_case_file_keeps_its_class_name() {
        assert_eq!(
            ClassNameFromFile::class_of(Path::new("client/remotes/OrdersClient.ts")),
            "OrdersClient"
        );
    }
}
