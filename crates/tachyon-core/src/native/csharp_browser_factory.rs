use super::registry::NativeCompanionInput;
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use crate::project::NativeCompanion;
use std::path::Path;

pub(super) struct CsharpBrowserFactory;

impl CsharpBrowserFactory {
    pub(super) fn prepare(
        item: &NativeCompanionInput,
        source: &str,
        path: &Path,
    ) -> Result<Option<String>, Failure> {
        Self::selected(item, source, path)
    }

    pub(super) fn table(
        table: &str,
        factory: Option<&str>,
        class: &str,
        path: &Path,
    ) -> Result<String, Failure> {
        factory
            .map(|factory| Self::written(table, factory, class, path))
            .transpose()
            .map(|written| written.unwrap_or_else(|| table.to_owned()))
    }

    fn selected(
        item: &NativeCompanionInput,
        source: &str,
        path: &Path,
    ) -> Result<Option<String>, Failure> {
        if item.web_assembly && item.language == NativeCompanion::CSharp {
            crate::behavior::csharp_browser_factory(path, source, &item.class)
        } else {
            Ok(None)
        }
    }

    fn written(table: &str, factory: &str, class: &str, path: &Path) -> Result<String, Failure> {
        let marker = format!("var Instance = new {class}()");
        Self::unique(table, &marker, path)?;
        Ok(table.replacen(&marker, factory, 1))
    }

    fn unique(table: &str, marker: &str, path: &Path) -> Result<(), Failure> {
        if table.matches(marker).count() != 1 {
            return Err(Failure::one(diagnostic(
                1406,
                "The C# browser Service table requires exactly one generated creation boundary.",
                Some(String::from(
                    "Preserve the generated Service factory; no provider may be silently omitted.",
                )),
                source_span(&path.to_string_lossy(), 0, 1),
            )));
        }
        Ok(())
    }
}
