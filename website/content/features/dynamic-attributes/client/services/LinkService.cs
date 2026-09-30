using System.Collections.Generic;

[Service]
public class LinkService
{
    public object Topic = new Dictionary<string, string> { ["slug"] = "routing", ["title"] = "Routing" };
    public string State = "ready";
    public string Message = "Everything is up to date.";
}
