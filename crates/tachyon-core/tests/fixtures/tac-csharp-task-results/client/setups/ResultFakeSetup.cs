[Setup]
public sealed class ResultFakeSetup
{
    public static BehaviorAdapters configure(BehaviorExecution execution)
    {
        execution.Check();
        var adapters = new BehaviorAdapters();
        adapters.http["results"] = new ResultAnswerHelper();
        return adapters;
    }
}
