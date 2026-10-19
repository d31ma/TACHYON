[Service]
public class ActivityService
{
    public string State = "foreground";

    [OnEvent({"replay":true})]
    public void ApplicationStateChanged(ApplicationStateChanged changed)
    {
        State = changed.State;
    }
}
