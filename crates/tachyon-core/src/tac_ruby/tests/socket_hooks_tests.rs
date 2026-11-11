use super::Page;
use crate::tac_web::WebLanguage;
use crate::tac_web::socket_hooks_harness::{SocketHooks, SocketHooksCase};

struct RubyHooks;

impl RubyHooks {
    const FILES: [(&'static str, &'static str); 2] = [
        (
            "client/models/ChatMessage.rb",
            "# @!attribute [r] text\n#   @return [String]\nChatMessage = Data.define(:text)\n",
        ),
        (
            "client/remotes/ChatClient.rb",
            "#[HTTPClient(\"/chat\")]\nclass ChatClient\n  # @param message [ChatMessage]\n  #[SocketRequest]\n  def receive(message)\n  end\nend\n",
        ),
    ];

    const SERVICE: &'static str = "#[Service]\nclass ChatService\n  #[Needs]\n  # @return [ChatClient]\n  attr_reader :chat\n\n  # @return [Integer]\n  attr_reader :opens\n  # @return [Integer]\n  attr_reader :reopens\n  # @return [Float]\n  attr_reader :code\n  # @return [Boolean]\n  attr_reader :retrying\n  # @return [Integer]\n  attr_reader :ticks\n\n  \
         def initialize\n    @opens = 0\n    @reopens = 0\n    @code = 0.0\n    @retrying = false\n    @ticks = 0\n  end\n\n  \
         # @param opened [SocketOpened]\n  #[OnEvent]\n  def connected(opened)\n    @opens += 1\n    @reopens += opened.reconnected ? 1 : 0\n  end\n\n  \
         # @param closed [SocketClosed]\n  #[OnEvent]\n  def disconnected(closed)\n    @code = closed.code\n    @retrying = closed.reconnecting\n  end\n\n  \
         #[Every(\"30s\")]\n  def refresh\n    @ticks += 1\n  end\nend\n";
}

#[tokio::test]
async fn a_ruby_service_hears_its_socket_and_runs_its_every_method() {
    if let Some(reason) = Page::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let case = SocketHooksCase {
        language: WebLanguage::Ruby,
        extension: "rb",
        files: &RubyHooks::FILES,
        service: RubyHooks::SERVICE,
        sockets: true,
    };
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    let report = SocketHooks::run(root.path(), &case).await;
    SocketHooks::check(&report, true);
}
