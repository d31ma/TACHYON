use serde_json::{Value, json};

use super::SortedJson;

fn parsed(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or_else(|error| panic!("{error}"))
}

#[test]
fn sorted_text_orders_keys_at_every_depth() {
    let authored = parsed(r#"{"b":1,"a":{"d":1,"c":[{"f":1,"e":2}]}}"#);
    assert_eq!(
        SortedJson::text(&authored),
        r#"{"a":{"c":[{"e":2,"f":1}],"d":1},"b":1}"#
    );
    assert_eq!(SortedJson::value(&authored), authored);
}

#[test]
fn sorted_text_writes_scalars_and_strings_as_serde_json_does() {
    let mixed =
        parsed(r#"{"q":"say \"hi\"\n","n":-1.5,"e":[],"o":{},"z":null,"t":true,"u":"\u00e9"}"#);
    assert_eq!(
        SortedJson::text(&mixed),
        serde_json::to_string(&SortedJson::value(&mixed)).unwrap_or_default()
    );
}

#[test]
fn sorted_pretty_text_orders_keys() {
    let authored = parsed(r#"{"b":1,"a":2}"#);
    assert_eq!(
        SortedJson::pretty(&authored),
        "{\n  \"a\": 2,\n  \"b\": 1\n}"
    );
}

#[test]
fn entries_come_out_sorted_by_key() {
    let authored = parsed(r#"{"draft":1,"darkMode":2,"Zone":3}"#);
    let Value::Object(map) = authored else {
        panic!("object");
    };
    let entries: Vec<(&String, &Value)> = SortedJson::entries(&map);
    let keys: Vec<&str> = entries.iter().map(|(key, _)| key.as_str()).collect();
    assert_eq!(keys, ["Zone", "darkMode", "draft"]);
    assert_eq!(SortedJson::keys(&map), ["Zone", "darkMode", "draft"]);
    assert_eq!(SortedJson::values(&map), [&json!(3), &json!(2), &json!(1)]);
    assert_eq!(entries[1], (&"darkMode".to_owned(), &json!(2)));
}

#[cfg(feature = "json-preserve-order")]
#[test]
fn the_json_order_guard_build_keeps_authored_order() {
    let authored = parsed(r#"{"b":1,"a":2}"#);
    assert_eq!(authored.to_string(), r#"{"b":1,"a":2}"#);
    assert_eq!(SortedJson::text(&authored), r#"{"a":2,"b":1}"#);
}
