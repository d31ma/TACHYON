use super::LuaProgram;
use serde_json::json;

#[test]
fn lua_json_keeps_nulls_and_empty_objects_and_refuses_invalid_input() {
    let Some(printed) = LuaProgram::printed(&[
        include_str!("lua_json.lua"),
        include_str!("../../tests/fixtures/lua/json_contract.lua"),
    ]) else {
        return;
    };
    assert!(printed.contains("Lua JSON contracts passed"), "{printed}");
}

#[test]
fn lua_behavior_timeouts_use_wall_time_and_waits_do_not_spin() {
    let Some(printed) = LuaProgram::printed(&[
        include_str!("lua_json.lua"),
        include_str!("lua_behaviors.lua"),
        include_str!("../../tests/fixtures/behavior/lua_clock_contract.lua"),
    ]) else {
        return;
    };
    assert!(
        printed.contains("Lua behavior clock contracts passed"),
        "{printed}"
    );
}

#[test]
fn a_yon_lua_socket_reports_a_deep_message_and_keeps_a_null_message() {
    let Some(program) = LuaProgram::new() else {
        return;
    };
    let runtime = program.file(
        "runtime.lua",
        &[
            include_str!("lua_json.lua"),
            include_str!("../handler/preludes/yon.lua"),
        ],
    );
    let handler = program.file(
        "handler.lua",
        &[include_str!("../../tests/fixtures/lua/socket_contract.lua")],
    );
    let deep = format!("{}{}", "[".repeat(100), "]".repeat(100));
    let frames = [
        json!({ "request_id": "r1", "method": "GET" }),
        json!({ "request_id": "r1", "kind": "message", "body": { "data": deep } }),
        json!({ "request_id": "r1", "kind": "message", "body": { "data": "null" } }),
        json!({ "request_id": "r1", "kind": "message", "body": { "data": "[1,null,2]" } }),
    ]
    .iter()
    .flat_map(LuaProgram::frame)
    .collect::<Vec<u8>>();
    let output = program.run(&[runtime, handler], &[("YON_SOCKET_FRAMES", "1")], &frames);
    let text = LuaProgram::text(&output);
    assert!(output.status.success(), "{text}");
    assert!(text.contains("messages=nil;[1,null,2]"), "{text}");
    assert!(
        text.contains(
            "errors=The socket message is not valid JSON, or it is nested more than 64 levels deep"
        ),
        "{text}"
    );
}
