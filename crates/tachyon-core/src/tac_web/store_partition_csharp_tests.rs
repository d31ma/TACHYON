#![allow(clippy::expect_used)]

use super::WebLanguage;
use super::fsharp_tests::fsharp_web_ready;
use super::store_partition_tests::StorePartitionProject;
use super::tests::{build_and_publish, run_driver, write_service};
use std::path::{Path, PathBuf};

struct CsharpStorePartition;

impl CsharpStorePartition {
    const FILES: [(&'static str, &'static str); 4] = [
        (
            "client/setups/ChatSetup.cs",
            "[Setup]\npublic sealed class ChatSetup\n{\n    public static BehaviorAdapters configure(BehaviorExecution execution) { return new BehaviorAdapters(); }\n}\n",
        ),
        (
            "client/entities/MessageEntity.cs",
            "[Entity({\"adapter\":\"indexeddb\",\"partition\":\"room\"})]\n[Table(\"messages\")]\npublic sealed class MessageEntity\n{\n    [Id]\n    public string id;\n    public string room;\n    public string text;\n}\n",
        ),
        (
            "client/stores/MessageStore.cs",
            "[Store(typeof(MessageEntity), {\"fields\":[\"id\",\"room\",\"text\"],\"defaultSize\":20,\"maxSize\":100,\"setup\":\"ChatSetup\"})]\npublic interface MessageStore\n{\n    System.Threading.Tasks.Task<BehaviorJSON> findByRoom(BehaviorJSON room);\n}\n",
        ),
        (
            "client/services/ChatService.cs",
            r#"using System.Linq;

[Service]
public class ChatService
{
    [Needs]
    MessageStore _messages;
    private readonly BehaviorContext behaviors = Behaviors.create(typeof(ChatSetup), "chat", new BehaviorSignal());
    public string Heard = "";

    public string Open(string room) => Finished(() => Messages().load(BehaviorJSON.From(room)));

    public bool IsOpen(string room) => Messages().loaded(BehaviorJSON.From(room)).GetAwaiter().GetResult();

    public string Close(string room) => Finished(() => Messages().unload(BehaviorJSON.From(room)));

    public string Add(string id, string room, string text) => Finished(() => Messages().save(Row(id, room, text)));

    public string Read(string room)
    {
        try
        {
            return Ids(_messages.findByRoom(BehaviorJSON.From(room)).GetAwaiter().GetResult());
        }
        catch (BehaviorError error)
        {
            return Failed(error);
        }
    }

    [OnEvent]
    public void HearLoaded(StorePartitionLoaded loaded)
    {
        Heard = loaded.Table + "/" + loaded.Partition + "/" + (long)loaded.Rows;
    }

    private BehaviorRepository Messages() => behaviors.repository(typeof(MessageEntity));

    private static string Failed(BehaviorError error) => "error:" + error.Code;

    private static string Finished(System.Func<System.Threading.Tasks.Task> work)
    {
        try
        {
            work().GetAwaiter().GetResult();
            return "";
        }
        catch (BehaviorError error)
        {
            return Failed(error);
        }
    }

    private static BehaviorJSON Row(string id, string room, string text) =>
        BehaviorJSON.Object(new System.Collections.Generic.Dictionary<string, BehaviorJSON> { ["id"] = BehaviorJSON.From(id), ["room"] = BehaviorJSON.From(room), ["text"] = BehaviorJSON.From(text) });

    private static string Ids(BehaviorJSON rows) =>
        string.Join(",", rows.Items.Select(row => row.Get("id").Text).OrderBy(id => id, System.StringComparer.Ordinal));
}
"#,
        ),
    ];

    fn written(root: &Path) -> PathBuf {
        let mut service = PathBuf::new();
        for (relative, text) in Self::FILES {
            service = write_service(root, relative, text);
        }
        service
    }
}

#[tokio::test]
async fn a_csharp_service_loads_one_partition_and_reads_it_from_memory() {
    if !fsharp_web_ready() {
        eprintln!("skipping: the .NET wasm-tools workload or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let service = CsharpStorePartition::written(root.path());
    StorePartitionProject::runtime_files(root.path());
    build_and_publish(
        root.path(),
        "chat",
        "ChatService",
        WebLanguage::CSharp,
        service,
    )
    .await;
    let report = run_driver(root.path(), StorePartitionProject::DRIVER);
    StorePartitionProject::assert_report(&report);
}
