internal sealed class TacTaskInvocation
{
    private readonly System.Threading.Tasks.Task<object> Work;
    private readonly System.Threading.CancellationToken Cancellation;
    private readonly Func<object, string> Finish;
    private readonly Action Release;

    internal TacTaskInvocation(System.Threading.Tasks.Task<object> work, System.Threading.CancellationToken cancellation, Func<object, string> finish, Action release)
    {
        Work = work;
        Cancellation = cancellation;
        Finish = finish;
        Release = release;
    }

    internal System.Threading.Tasks.Task<string> Run() => Complete();

    private async System.Threading.Tasks.Task<string> Complete()
    {
        try { return Finish(await Work.WaitAsync(Cancellation)); }
        finally { Release(); }
    }
}
