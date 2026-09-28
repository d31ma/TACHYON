use std::collections::BTreeMap;
use std::path::Path;

struct Declaration {
    start: usize,
    end: usize,
    name: String,
    suffix: &'static str,
    folder: &'static str,
}

impl Declaration {
    fn role_name(&self) -> String {
        if self.name.ends_with(self.suffix) {
            self.name.clone()
        } else {
            format!("{}{}", self.name, self.suffix)
        }
    }
}

pub fn write(root: &Path, relative: &str, source: &str) {
    let extension = Path::new(relative)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    if !matches!(
        extension,
        "js" | "ts" | "java" | "kt" | "cs" | "rs" | "py" | "php"
    ) {
        write_file(root, relative, source);
        return;
    }
    let declarations = declarations(source, extension);
    let names: BTreeMap<_, _> = declarations
        .iter()
        .map(|item| (item.name.clone(), item.role_name()))
        .collect();
    let entry = split_role_files(root, relative, source, extension, declarations, &names);
    write_file(root, relative, &rename(&entry, &names));
}

fn split_role_files(
    root: &Path,
    relative: &str,
    source: &str,
    extension: &str,
    declarations: Vec<Declaration>,
    names: &BTreeMap<String, String>,
) -> String {
    let parent = Path::new(relative)
        .parent()
        .expect("entry parent")
        .to_str()
        .expect("UTF-8 fixture path");
    let role_root = if parent == "client/services" {
        "client"
    } else if parent.starts_with("server/") {
        "server"
    } else {
        parent
    };
    let mut entry = String::new();
    let mut cursor = 0;
    for item in declarations {
        if parent.ends_with(&format!("/{}", item.folder)) {
            continue;
        }
        entry.push_str(&source[cursor..item.start]);
        entry.push('\n');
        cursor = item.end;
        let name = &names[&item.name];
        let folder_root = if item.folder == "controllers" {
            parent
        } else {
            role_root
        };
        let path = format!("{folder_root}/{}/{name}.{extension}", item.folder);
        write_file(root, &path, &rename(&source[item.start..item.end], names));
    }
    entry.push_str(&source[cursor..]);
    entry
}

fn write_file(root: &Path, relative: &str, source: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("fixture parent")).expect("fixture directories");
    std::fs::write(path, source).expect("fixture source");
}

fn declarations(source: &str, extension: &str) -> Vec<Declaration> {
    let masked = mask(source, extension == "py");
    let mut declarations = Vec::new();
    let mut offset = 0;
    let mut pending = None;
    let mut role = None;
    while offset < source.len() {
        let end = source[offset..]
            .find('\n')
            .map_or(source.len(), |length| offset + length + 1);
        let line = &source[offset..end];
        if line.starts_with('@')
            || line.starts_with("#[")
            || line.starts_with('[')
            || line.starts_with("// @")
        {
            pending.get_or_insert(offset);
            role = marker_role(line).or(role);
            offset = end;
            continue;
        }
        if let Some(name) = type_name(&masked[offset..end]) {
            let declaration_end = block_end(source, &masked, offset, end, extension);
            if let Some((suffix, folder)) = role.take() {
                declarations.push(Declaration {
                    start: pending.take().unwrap_or(offset),
                    end: declaration_end,
                    name: name.to_owned(),
                    suffix,
                    folder,
                });
            }
            pending = None;
            offset = declaration_end;
        } else {
            if !line.trim().is_empty() && !line.trim_start().starts_with("//") {
                pending = None;
                role = None;
            }
            offset = end;
        }
    }
    declarations
}

fn marker_role(line: &str) -> Option<(&'static str, &'static str)> {
    let marker = line
        .trim_start_matches(['@', '#', '['])
        .split(['(', ']', '\n', ' '])
        .next()?
        .trim_start_matches("yon::");
    match marker.to_ascii_lowercase().as_str() {
        "setup" => Some(("Setup", "setups")),
        "entity" => Some(("Entity", "entities")),
        "store" => Some(("Store", "stores")),
        "service" => Some(("Service", "services")),
        "controller" | "requestroute" => Some(("Controller", "controllers")),
        _ => None,
    }
}

fn type_name(line: &str) -> Option<&str> {
    let words: Vec<_> = line
        .split(|value: char| !value.is_ascii_alphanumeric() && value != '_')
        .filter(|word| !word.is_empty())
        .collect();
    let index = words
        .iter()
        .position(|word| matches!(*word, "class" | "struct" | "object"))?;
    if !words[..index].iter().all(|word| {
        matches!(
            *word,
            "public"
                | "private"
                | "internal"
                | "static"
                | "sealed"
                | "abstract"
                | "final"
                | "export"
                | "default"
                | "pub"
                | "open"
        )
    }) {
        return None;
    }
    words.get(index + 1).copied()
}

fn block_end(source: &str, masked: &str, start: usize, line_end: usize, extension: &str) -> usize {
    if extension == "py" {
        return python_block_end(source, line_end);
    }
    let mut end = brace_end(masked, start);
    if extension == "rs" {
        let tail = &masked[end..];
        let skipped = tail.len() - tail.trim_start().len();
        if tail.trim_start().starts_with("impl ") {
            end = brace_end(masked, end + skipped);
        }
    }
    end
}

fn python_block_end(source: &str, line_end: usize) -> usize {
    let mut end = line_end;
    while end < source.len() {
        let next = source[end..]
            .find('\n')
            .map_or(source.len(), |length| end + length + 1);
        let line = &source[end..next];
        if !line.trim().is_empty() && !line.starts_with(char::is_whitespace) {
            break;
        }
        end = next;
    }
    end
}

fn brace_end(masked: &str, start: usize) -> usize {
    let mut depth = 0;
    for (offset, byte) in masked.as_bytes()[start..].iter().enumerate() {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return start + offset + 1;
                }
            }
            b';' if depth == 0 => return start + offset + 1,
            _ => {}
        }
    }
    masked.len()
}

fn mask(source: &str, python: bool) -> String {
    let mut bytes = source.as_bytes().to_vec();
    let mut cursor = 0;
    while cursor < bytes.len() {
        let start = cursor;
        if matches!(bytes[cursor], b'\'' | b'"' | b'`') {
            cursor = quoted_end(&bytes, cursor);
        } else if bytes[cursor..].starts_with(b"//") || (python && bytes[cursor] == b'#') {
            cursor += bytes[cursor..]
                .iter()
                .position(|value| *value == b'\n')
                .unwrap_or(bytes.len() - cursor);
        } else if bytes[cursor..].starts_with(b"/*") {
            cursor = comment_end(&bytes, cursor);
        } else {
            cursor += 1;
            continue;
        }
        let end = cursor.min(bytes.len());
        for byte in &mut bytes[start..end] {
            if *byte != b'\n' {
                *byte = b' ';
            }
        }
    }
    String::from_utf8(bytes).expect("masked fixture")
}

fn quoted_end(bytes: &[u8], start: usize) -> usize {
    let quote = bytes[start];
    let mut cursor = start + 1;
    while cursor < bytes.len() {
        if bytes[cursor] == b'\\' {
            cursor += 2;
        } else if bytes[cursor] == quote {
            return cursor + 1;
        } else {
            cursor += 1;
        }
    }
    cursor
}

fn comment_end(bytes: &[u8], start: usize) -> usize {
    let mut cursor = start + 2;
    while cursor < bytes.len() && !bytes[cursor..].starts_with(b"*/") {
        cursor += 1;
    }
    cursor + 2
}

fn rename(source: &str, names: &BTreeMap<String, String>) -> String {
    let mut output = String::new();
    let bytes = source.as_bytes();
    let masked = mask(source, false);
    let mut cursor = 0;
    while cursor < bytes.len() {
        if bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'_' {
            let start = cursor;
            while cursor < bytes.len()
                && (bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'_')
            {
                cursor += 1;
            }
            let word = &source[start..cursor];
            output.push_str(if word == "Products" && masked.as_bytes()[start] == b' ' {
                word
            } else {
                names.get(word).map_or(word, String::as_str)
            });
        } else {
            let character = source[cursor..].chars().next().expect("fixture character");
            output.push(character);
            cursor += character.len_utf8();
        }
    }
    output
}
