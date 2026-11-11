[Service]
public sealed class OrderService
{
    [Needs]
    private readonly OrderClient _orderClient;

    public int Status = 0;
    public string Title = "";
    public int Mistakes = 0;

    public async System.Threading.Tasks.Task send()
    {
        var failure = new BehaviorError("none");
        try { await _orderClient.send(BehaviorJSON.Parse("{\"name\":\"\",\"level\":12}")); }
        catch (BehaviorError error) { failure = error; }
        failure.TryReadBody(out Models.Problem problem);
        Status = failure.Status;
        Title = problem?.Title ?? "";
        Mistakes = problem?.Errors?.Length ?? 0;
    }
}
