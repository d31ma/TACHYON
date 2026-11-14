use super::locks_tests::LocksTests;
use super::{WebApis, WebLanguage, lua_unavailable};

struct LocksLuaTests;

impl LocksLuaTests {
    const LUA: &str = r#"-- @Service
---@class StampService
StampService = {
  ---@type string
  held = "",
}

---@return nil
function StampService:take()
  Locks:request("outbox")
end

---@return nil
function StampService:give()
  Locks:release("outbox")
end

---@param granted LockGranted
---@return nil
-- @OnEvent
function StampService:hear_granted(granted)
  self.held = granted.name
end
"#;
}

#[tokio::test]
async fn a_lua_web_service_takes_and_releases_a_lock() {
    if lua_unavailable() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Lua, "lua", LocksLuaTests::LUA).await;
    LocksTests::assert_held(root.path(), true);
}
