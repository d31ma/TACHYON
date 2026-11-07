[Service]
public class PlaceService
{
    private readonly Geolocation geolocation = new();
    private readonly Clipboard clipboard = new();
    public string Place = "";
    public string Problem = "";

    public void Locate() => KeepProblem(geolocation.CurrentPosition);

    public void Copy() => KeepProblem(() => clipboard.WriteText(Place));

    [OnEvent]
    public void Found(PositionFound found)
    {
        Place = $"{found.Latitude}, {found.Longitude}";
    }

    [OnEvent]
    public void Failed(BrowserCallFailed failed)
    {
        Problem = failed.Message;
    }

    private void KeepProblem(Action call)
    {
        try
        {
            call();
        }
        catch (InvalidOperationException refusal)
        {
            Problem = refusal.Message;
        }
    }
}
