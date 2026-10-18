use super::csharp_stream_tests::{CsharpStreams, Sources};

struct FsharpStreams;

impl FsharpStreams {
    const FILES: [(&str, &str); 4] = [
        (
            "client/models/Tick.fs",
            "type Tick = { Sku: string; Level: int }\n",
        ),
        (
            "client/models/ChatMessage.fs",
            "type ChatMessage = { Text: string }\n",
        ),
        (
            "client/remotes/TicksClient.fs",
            "open Models\n\n[<HTTPClient(\"/ticks\")>]\ntype TicksClient =\n    [<StreamRequest>]\n    abstract Receive: Tick -> unit\n",
        ),
        (
            "client/remotes/ChatClient.fs",
            "open Models\n\n[<HTTPClient(\"/chat\")>]\ntype ChatClient =\n    [<SocketRequest>]\n    abstract Receive: ChatMessage -> unit\n",
        ),
    ];

    const SERVICES: [(&str, &str, &str); 2] = [
        (
            "ticks",
            "TicksService",
            "open System.Threading.Tasks\nopen Models\n\n[<Service>]\ntype TicksService() =\n    [<Needs>]\n    member val TicksClient : TicksClient = Unchecked.defaultof<TicksClient> with get, set\n\n    [<Needs>]\n    member val ChatClient : ChatClient = Unchecked.defaultof<ChatClient> with get, set\n\n    member val Level : int = 0 with get, set\n    member val Said : string = \"\" with get, set\n    member val Ticks : Tick list = [] with get, set\n\n    \
             [<OnEvent>]\n    member this.Heard(tick: Tick) : unit =\n        this.Level <- tick.Level\n        this.Ticks <- this.Ticks @ [ tick ]\n\n    \
             [<OnEvent>]\n    member this.Answered(message: ChatMessage) : unit =\n        this.Said <- message.Text\n\n    \
             member this.Say(text: string) : Task =\n        this.ChatClient.Send({ Text = text })\n",
        ),
        (
            "quiet",
            "QuietService",
            "[<Service>]\ntype QuietService() =\n    member val Count : int = 0 with get, set\n",
        ),
    ];
}

#[tokio::test]
async fn an_fsharp_service_receives_its_client_streams_as_events_and_sends_on_its_socket() {
    let sources = Sources {
        files: &FsharpStreams::FILES,
        services: &FsharpStreams::SERVICES,
        extension: "fs",
    };
    if let Some(report) = CsharpStreams::report(&sources).await {
        CsharpStreams::check(&report);
    }
}
