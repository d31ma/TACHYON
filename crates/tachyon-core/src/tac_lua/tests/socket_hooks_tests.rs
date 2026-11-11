use super::Page;
use crate::tac_web::WebLanguage;
use crate::tac_web::socket_hooks_shared_tests::{SocketHooks, SocketHooksCase};

struct LuaHooks;

impl LuaHooks {
    const FILES: [(&'static str, &'static str); 2] = [
        (
            "client/models/ChatMessage.lua",
            "---@class ChatMessage\n---@field text string\nChatMessage = {}\n",
        ),
        (
            "client/remotes/ChatClient.lua",
            "-- @HTTPClient({\"path\":\"/chat\"})\n---@class ChatClient\nChatClient = {}\n\n---@param message ChatMessage\n---@return nil\n-- @SocketRequest\nfunction ChatClient:receive(message)\nend\n",
        ),
    ];

    const SERVICE: &'static str = "-- @Service\n---@class ChatService\nChatService = {\n  -- @Needs\n  ---@type ChatClient\n  chat = nil,\n  ---@type integer\n  opens = 0,\n  ---@type integer\n  reopens = 0,\n  ---@type number\n  code = 0,\n  ---@type boolean\n  retrying = false,\n  ---@type integer\n  ticks = 0,\n}\n\n\
         ---@param opened SocketOpened\n---@return nil\n-- @OnEvent\nfunction ChatService:connected(opened)\n  self.opens = self.opens + 1\n  if opened.reconnected then\n    self.reopens = self.reopens + 1\n  end\nend\n\n\
         ---@param closed SocketClosed\n---@return nil\n-- @OnEvent\nfunction ChatService:disconnected(closed)\n  self.code = closed.code\n  self.retrying = closed.reconnecting\nend\n\n\
         ---@return nil\n-- @Every(\"30s\")\nfunction ChatService:refresh()\n  self.ticks = self.ticks + 1\nend\n";
}

#[tokio::test]
async fn a_lua_service_hears_its_socket_and_runs_its_every_method() {
    if let Some(reason) = Page::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let case = SocketHooksCase {
        language: WebLanguage::Lua,
        extension: "lua",
        files: &LuaHooks::FILES,
        service: LuaHooks::SERVICE,
        sockets: true,
    };
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    let report = SocketHooks::run(root.path(), &case).await;
    SocketHooks::check(&report, true);
}
