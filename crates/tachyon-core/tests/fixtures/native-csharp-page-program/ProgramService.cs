[Service]
public sealed class __CLASS__
{
    [Needs]
    private readonly PageProgramClient _pageProgramClient;

    public string Identity = "";
    public int Count = 0;

    public async System.Threading.Tasks.Task Inspect()
    {
        Show(await _pageProgramClient.read());
    }

    public async System.Threading.Tasks.Task Increment()
    {
        Show(await _pageProgramClient.increment());
    }

    private void Show(Models.ProgramState state)
    {
        Identity = state.Identity;
        Count = state.Count;
    }
}
