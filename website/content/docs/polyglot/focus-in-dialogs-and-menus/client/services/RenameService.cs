[Service]
public class RenameService
{
    private readonly Focus focus = new();
    public bool Editing = false;
    public string Problem = "";

    public void Open()
    {
        Editing = true;
        KeepProblem(() => focus.Trap("dialog"));
        KeepProblem(() => focus.Move("name"));
    }

    public void Close()
    {
        Editing = false;
        KeepProblem(focus.Release);
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
