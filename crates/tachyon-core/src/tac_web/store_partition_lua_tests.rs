#![allow(clippy::expect_used)]

use super::WebLanguage;
use super::store_partition_tests::StorePartitionProject;
use super::tests::{build_and_publish, node_available, run_driver, write_service};
use std::path::{Path, PathBuf};

struct LuaStorePartition;

impl LuaStorePartition {
    const FILES: [(&'static str, &'static str); 4] = [
        (
            "client/setups/ChatSetup.lua",
            "-- @Setup\n---@class ChatSetup\nChatSetup = {}\n\n---@param execution table\n---@return table\nfunction ChatSetup.configure(execution)\n  return {}\nend\n",
        ),
        (
            "client/entities/MessageEntity.lua",
            "-- @Entity({\"adapter\":\"indexeddb\",\"partition\":\"room\"})\n-- @Table(\"messages\")\n---@class MessageEntity\nMessageEntity = {\n  -- @Id\n  ---@type string\n  id = nil,\n  ---@type string\n  room = nil,\n  ---@type string\n  text = nil,\n}\n",
        ),
        (
            "client/stores/MessageStore.lua",
            "-- @Store(MessageEntity, {\"fields\":[\"id\",\"room\",\"text\"],\"defaultSize\":20,\"maxSize\":100,\"setup\":\"ChatSetup\"})\n---@class MessageStore\nMessageStore = {}\n\n---@param room string\n---@return table[]\nfunction MessageStore:find_by_room(room)\nend\n",
        ),
        (
            "client/services/ChatService.lua",
            r#"-- @Service
---@class ChatService
ChatService = {
  -- @Needs
  ---@type MessageStore
  messages = nil,
  ---@type string
  heard = "",
}

---@return nil
-- @OnStart
function ChatService:start()
  self.behaviors = Behaviors.create(ChatSetup, { partition = "chat", signal = BehaviorSignal.new() })
end

---@param room string
---@return string
function ChatService:open(room)
  local repository = self.behaviors:repository(MessageEntity)
  local ok, failure = pcall(repository.load, repository, room)
  return ok and "" or "error:" .. failure.behavior_code
end

---@param room string
---@return boolean
function ChatService:is_open(room)
  local repository = self.behaviors:repository(MessageEntity)
  local ok, loaded = pcall(repository.loaded, repository, room)
  return ok and loaded
end

---@param room string
---@return string
function ChatService:close(room)
  local repository = self.behaviors:repository(MessageEntity)
  local ok, failure = pcall(repository.unload, repository, room)
  return ok and "" or "error:" .. failure.behavior_code
end

---@param id string
---@param room string
---@param text string
---@return string
function ChatService:add(id, room, text)
  local repository = self.behaviors:repository(MessageEntity)
  local ok, failure = pcall(repository.save, repository, { id = id, room = room, text = text })
  return ok and "" or "error:" .. failure.behavior_code
end

---@param room string
---@return string
function ChatService:read(room)
  local ok, rows = pcall(self.messages.find_by_room, self.messages, room)
  if not ok then
    return "error:" .. rows.behavior_code
  end
  local ids = {}
  for index, row in ipairs(rows) do
    ids[index] = row.id
  end
  table.sort(ids)
  return table.concat(ids, ",")
end

---@param loaded StorePartitionLoaded
---@return nil
-- @OnEvent
function ChatService:hear_loaded(loaded)
  self.heard = loaded.table .. "/" .. loaded.partition .. "/" .. math.floor(loaded.rows)
end
"#,
        ),
    ];

    fn skip_reason() -> Option<String> {
        if !node_available() {
            return Some(String::from("node is unavailable"));
        }
        let reason = crate::tac_lua::LuaRuntimeBuild::unavailable_reason();
        assert!(
            reason.is_none() || std::env::var_os("CI").is_none(),
            "CI installs Emscripten to build the pinned Lua runtime, so the Lua Tac tests must run there: {reason:?}"
        );
        reason
    }

    fn written(root: &Path) -> PathBuf {
        let mut service = PathBuf::new();
        for (relative, text) in Self::FILES {
            service = write_service(root, relative, text);
        }
        service
    }
}

#[tokio::test]
async fn a_lua_service_loads_one_partition_and_reads_it_from_memory() {
    if let Some(reason) = LuaStorePartition::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let service = LuaStorePartition::written(root.path());
    StorePartitionProject::runtime_files(root.path());
    build_and_publish(
        root.path(),
        "chat",
        "ChatService",
        WebLanguage::Lua,
        service,
    )
    .await;
    let report = run_driver(root.path(), StorePartitionProject::DRIVER);
    StorePartitionProject::assert_report(&report);
}
