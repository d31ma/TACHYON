[Service]
public sealed class EditorService {
    [SessionState]
    public string Draft = "";
    [LocalState]
    public string Theme = "light";
}
