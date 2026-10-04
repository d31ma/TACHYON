[Service]
public class ReportService
{
    private readonly WindowControls windowControls = new();
    public string Problem = "";

    public void Minimize() => KeepProblem(windowControls.Minimize);

    public void Resize() => KeepProblem(() => windowControls.Resize(1280, 800));

    public void Rename() => KeepProblem(() => windowControls.SetTitle("Quarterly report"));

    private void KeepProblem(Action call)
    {
        try { call(); }
        catch (InvalidOperationException refusal) { Problem = refusal.Message; }
    }
}
