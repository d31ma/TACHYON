[Service]
public class EditorService
{
    [SessionState]
    public string Draft = "";
    [LocalState]
    public string Theme = "calm";
}
