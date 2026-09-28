use std::fs;
use std::path::Path;

fn write(root: &Path, relative: &str, source: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("fixture parent")).expect("fixture directory");
    fs::write(path, source).expect("fixture source");
}

fn primary_role(line: &str) -> Option<&'static str> {
    let line = line.strip_prefix("// ").unwrap_or(line);
    if line.to_ascii_lowercase().starts_with("@yon::service") {
        return Some("Service");
    }
    if line.to_ascii_lowercase().starts_with("@yon::store") {
        return Some("Store");
    }
    [
        ("@RequestRoute", "Controller"),
        ("@request_route", "Controller"),
        ("#[RequestRoute", "Controller"),
        ("[RequestRoute", "Controller"),
        ("@Service", "Service"),
        ("@service", "Service"),
        ("#[Service", "Service"),
        ("[Service", "Service"),
        ("#[yon::service", "Service"),
        ("@Store", "Store"),
        ("@store", "Store"),
        ("#[Store", "Store"),
        ("[Store", "Store"),
        ("#[yon::store", "Store"),
    ]
    .into_iter()
    .find_map(|(marker, role)| line.starts_with(marker).then_some(role))
}

fn fixture_roles(source: &str) -> Vec<(usize, &'static str)> {
    let mut offset = 0;
    let mut roles = Vec::new();
    let mut imports = None;
    for line in source.split_inclusive('\n') {
        if line.starts_with("import ") || line.starts_with("from models import ") {
            imports.get_or_insert(offset);
        } else if let Some(role) = primary_role(line) {
            roles.push((imports.unwrap_or(offset), role));
            imports = None;
        } else if !line.starts_with("#[") && !line.starts_with("// @") {
            imports = None;
        }
        offset += line.len();
    }
    roles
}

fn role_name(body: &str) -> &str {
    for line in body.lines() {
        let declaration = line
            .strip_prefix("class ")
            .or_else(|| line.strip_prefix("export default class "))
            .or_else(|| line.strip_prefix("export class "))
            .or_else(|| line.strip_prefix("struct "))
            .or_else(|| line.strip_prefix("type "));
        let Some(declaration) = declaration else {
            continue;
        };
        if let Some(name) = declaration
            .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
            .next()
        {
            return name;
        }
    }
    panic!("known fixture class or struct");
}

fn folder(role: &str) -> &'static str {
    match role {
        "Controller" => "controllers",
        "Service" => "services",
        "Store" => "stores",
        _ => unreachable!("fixture role list is closed"),
    }
}

pub fn write_fixture(root: &Path, relative: &str, contents: &str) -> String {
    let extension = relative.rsplit_once('.').expect("fixture extension").1;
    let source = contents.trim_end().trim_end_matches("?>");
    let roles = fixture_roles(source);
    assert!(!roles.is_empty(), "fixture must explicitly declare a role");
    let first = roles.first().expect("role").0;
    let prefix = source
        .get(..first)
        .expect("fixture prefix")
        .trim_start()
        .trim_start_matches("<?php");
    let mut controller = None;
    for (index, &(start, role)) in roles.iter().enumerate() {
        let end = roles.get(index + 1).map_or(source.len(), |next| next.0);
        let body = source.get(start..end).expect("fixture role block");
        let name = role_name(body);
        assert!(name.ends_with(role), "fixture {name} must end with {role}");
        let path = format!("server/{}/{name}.{extension}", folder(role));
        let header = if role == "Controller" { prefix } else { "" };
        let opening = if extension == "php" { "<?php\n" } else { "" };
        write(root, &path, &format!("{opening}{header}{body}"));
        if role == "Controller" {
            controller.get_or_insert(path);
        }
    }
    controller.unwrap_or_else(|| relative.to_owned())
}
