pub(super) struct RustErrorPlace;

impl RustErrorPlace {
    const ARROW: &'static str = "--> ";
    const MODULE: &'static str = "mod model_";
    const DOC_START: &'static str = "#[doc = \"";
    const DOC_END: &'static str = "\"]";

    pub(super) fn model_file(detail: &str, source_name: &str, program: &str) -> Option<String> {
        let line = Self::first_error_line(detail, source_name)?;
        let lines: Vec<&str> = program.lines().take(line).collect();
        let header = lines
            .iter()
            .rposition(|text| text.starts_with(Self::MODULE))?;
        let path = lines
            .get(header.checked_sub(1)?)?
            .strip_prefix(Self::DOC_START)?
            .strip_suffix(Self::DOC_END)?;
        Some(path.to_owned())
    }

    fn first_error_line(detail: &str, source_name: &str) -> Option<usize> {
        let place = detail
            .lines()
            .find_map(|line| line.trim().strip_prefix(Self::ARROW))?;
        let mut parts = place.rsplitn(3, ':');
        let _column = parts.next()?;
        let line = parts.next()?.parse().ok()?;
        (parts.next()? == source_name).then_some(line)
    }
}

#[cfg(test)]
mod tests {
    use super::RustErrorPlace;

    const PROGRAM: &str = "struct ItemsController;\n\n#[allow(dead_code, unused_imports)]\npub mod models {\npub use self::model_0::*;\npub use self::model_1::*;\n#[doc = \"server/models/Item.rs\"]\nmod model_0 {\npub struct Item;\n}\n#[doc = \"server/models/ItemList.rs\"]\nmod model_1 {\npub struct ItemList {\n    pub items: Vec<Item>,\n}\n}\n}\n";

    fn detail(line: usize) -> String {
        format!(
            "error[E0412]: cannot find type `Item` in this scope\n  --> ItemsController.rs:{line}:20\n   |\n"
        )
    }

    #[test]
    fn an_error_inside_a_model_names_the_model_file() {
        let place = RustErrorPlace::model_file(&detail(14), "ItemsController.rs", PROGRAM);
        assert_eq!(place.as_deref(), Some("server/models/ItemList.rs"));
    }

    #[test]
    fn an_error_in_the_controller_or_another_file_names_no_model() {
        assert_eq!(
            RustErrorPlace::model_file(&detail(1), "ItemsController.rs", PROGRAM),
            None
        );
        assert_eq!(
            RustErrorPlace::model_file(&detail(14), "Other.rs", PROGRAM),
            None
        );
    }
}
