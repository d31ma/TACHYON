using Models;

[Service]
public class LinkService
{
    public Topic Topic = new Topic("routing", "Routing");
    public string State = "ready";
    public string Message = "Everything is up to date.";
}
