use super::{
    Module, erase_exports, open_parent, read_module_text, register_names, validate_imports,
};
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use crate::lua_source::LuaView;
use cap_std::fs::Dir;
use std::collections::BTreeSet;
use std::path::Path;

pub(super) fn reject_role_file(path: &Path, source: &str) -> Result<(), Failure> {
    if crate::annotation_layout::has_test_annotation(path, source)? {
        return Err(invalid(
            path,
            "Test and TestSetup belong in tests/, never in a production role folder.",
        ));
    }
    Ok(())
}

pub(super) fn include(
    root: &Dir,
    entry: &Path,
    source: &str,
    extension: &str,
    environment: &str,
    modules: &mut Vec<Module>,
    total: usize,
) -> Result<(), Failure> {
    if !entry.starts_with("client/tests") && !entry.starts_with("server/tests") {
        return Ok(());
    }
    let Some(name) = LuaView::either(entry, source, |path, text| {
        crate::behavior::test_configuration_name(path, text, environment)
    })?
    else {
        return Ok(());
    };
    let path = entry.with_file_name(format!("{name}.{extension}"));
    let mut names = declared_names(entry, source, modules)?;
    if !exists(root, &path)? {
        if production_configuration(modules, &name, environment)? {
            return Ok(());
        }
        return Err(invalid(
            &path,
            "The named test configuration is missing. Declare it in this sibling file, or name a discovered production Setup.",
        ));
    }
    let text = read_module_text(root, &path, Module::MAX_SOURCE.saturating_sub(total))?;
    validate(&path, &text, &name, environment)?;
    register_names(&path, &text, &mut names)?;
    validate_imports(&path, &text, extension)?;
    if modules.len() == Module::MAX_MODULES {
        return Err(invalid(
            &path,
            "An entry may compose at most 128 annotated files, including its test configuration.",
        ));
    }
    modules.push(Module {
        path: path.clone(),
        source: erase_exports(&path, text, extension)?,
    });
    Ok(())
}

fn production_configuration(
    modules: &[Module],
    name: &str,
    environment: &str,
) -> Result<bool, Failure> {
    for module in modules {
        let names = crate::annotation_layout::declaration_names(&module.path, &module.source)?;
        if names.iter().any(|declared| declared == name) {
            return LuaView::either(&module.path, &module.source, |path, text| {
                crate::behavior::is_production_configuration(path, text, environment, name)
            });
        }
    }
    Ok(false)
}

fn declared_names(
    entry: &Path,
    source: &str,
    modules: &[Module],
) -> Result<BTreeSet<String>, Failure> {
    let mut names = BTreeSet::new();
    register_names(entry, source, &mut names)?;
    for module in modules {
        register_names(&module.path, &module.source, &mut names)?;
    }
    Ok(names)
}

fn exists(root: &Dir, path: &Path) -> Result<bool, Failure> {
    let directory = open_parent(root, path)?;
    let name = path.file_name().unwrap_or_default();
    let cannot_inspect = || invalid(path, "Cannot inspect the named test configuration.");
    let entries = directory.entries().map_err(|_| cannot_inspect())?;
    for entry in entries {
        let entry = entry.map_err(|_| cannot_inspect())?;
        if entry.file_name() == name {
            return Ok(true);
        }
    }
    Ok(false)
}

fn validate(path: &Path, source: &str, name: &str, environment: &str) -> Result<(), Failure> {
    crate::annotation_layout::validate(path, source)?;
    let declarations = crate::annotation_layout::declaration_names(path, source)?;
    if declarations.as_slice() != [name] {
        return Err(invalid(
            path,
            "A test configuration file declares exactly the Setup class that Test names, and the filename matches that class.",
        ));
    }
    let marked = LuaView::either(path, source, |path, text| {
        crate::behavior::is_test_configuration(path, text, environment)
    })?;
    if !marked {
        return Err(invalid(
            path,
            "The named sibling configuration must carry TestSetup. Production Setup types belong in setups/.",
        ));
    }
    Ok(())
}

fn invalid(path: &Path, message: &str) -> Failure {
    Failure::one(diagnostic(
        1310,
        message,
        Some("Place one TestSetup class in a same-language file named after that class, beside the selecting test unit in client/tests/ or server/tests/.".into()),
        source_span(&path.to_string_lossy(), 0, 0),
    ))
}
