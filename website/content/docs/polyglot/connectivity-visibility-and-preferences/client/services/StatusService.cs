[Service]
public class StatusService
{
    public bool Online = true;
    public string Scheme = "light";

    [OnEvent({"replay":true})]
    public void ConnectivityChanged(ConnectivityChanged changed)
    {
        Online = changed.Online;
    }

    [OnEvent({"replay":true})]
    public void PreferencesChanged(PreferencesChanged changed)
    {
        Scheme = changed.Scheme;
    }
}
