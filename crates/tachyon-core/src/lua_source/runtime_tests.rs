use super::LuaProgram;

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
