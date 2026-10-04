[Service]
class EventService {
  private const string Collection = "hd-events";
  private static readonly object Deploys = new { kind = "deploy" };
  private static readonly object Change = new { count = 7 };
  private const int PageSize = 2;
  private const string Topic = "app.render";
  private const string Group = "renderers";

  [Needs]
  DataSource _dataSource;

  public System.Collections.Generic.Dictionary<string, object> Report() {
    _dataSource.CreateCollection(Collection);
    var id = _dataSource.Insert(Collection, Deploys).GetProperty("id").GetString()!;
    _dataSource.Insert(Collection, Deploys);
    _dataSource.Insert(Collection, Deploys);
    var found = _dataSource.FindById(Collection, id);
    var patched = _dataSource.Patch(Collection, id, Change);
    var report = CountPages();
    report["found"] = found.GetProperty("kind").GetString()!;
    report["patched"] = patched.GetProperty("count").GetInt32();
    report["job"] = RunJob();
    _dataSource.DeleteById(Collection, id);
    report["gone"] = _dataSource.FindById(Collection, id).ValueKind == System.Text.Json.JsonValueKind.Null;
    report["dropped"] = _dataSource.DropCollection(Collection).GetBoolean();
    return report;
  }

  private System.Collections.Generic.Dictionary<string, object> CountPages() {
    int pages = 0;
    int seen = 0;
    string? cursor = null;
    bool more = true;
    while (more) {
      var window = _dataSource.Scroll(Collection, Deploys, cursor, PageSize);
      pages += 1;
      seen += window.GetProperty("content").GetArrayLength();
      cursor = window.GetProperty("nextCursor").GetString();
      more = cursor != null;
    }
    return new System.Collections.Generic.Dictionary<string, object> { ["pages"] = pages, ["seen"] = seen };
  }

  private System.Text.Json.JsonElement RunJob() {
    _dataSource.Publish(Topic, new { app = "web" }, "render:web");
    var delivery = _dataSource.Claim(Topic, Group, 1, 30000)[0];
    _dataSource.Ack(Topic, Group, delivery);
    return delivery.GetProperty("payload");
  }
}
