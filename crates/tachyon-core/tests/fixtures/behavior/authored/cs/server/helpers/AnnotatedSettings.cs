[Settings({"prefix":"YON_APP_"})]
public sealed class AnnotatedSettings {
    public string title;
    [Min(1)]
    [Max(100)]
    public int size = 20;
    private AnnotatedSettings() {
        throw new System.Exception("must not instantiate");
    }
}

