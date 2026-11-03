internal sealed class TacHttpInvocation
{
    private readonly string request;
    private readonly System.Threading.CancellationToken signal;
    private readonly System.Threading.CancellationToken owner;

    private TacHttpInvocation(string request, System.Threading.CancellationToken signal, System.Threading.CancellationToken owner)
    {
        this.request = request;
        this.signal = signal;
        this.owner = owner;
    }

    internal static System.Threading.Tasks.Task<string> Run(string request, System.Threading.CancellationToken signal, System.Threading.CancellationToken owner) =>
        new TacHttpInvocation(request, signal, owner).Exchange();

    private async System.Threading.Tasks.Task<string> Exchange()
    {
        using var cancellation = System.Threading.CancellationTokenSource.CreateLinkedTokenSource(signal, owner);
        cancellation.Token.ThrowIfCancellationRequested();
        var ticket = TacHttpTransport.Start(request);
        return await Await(ticket, cancellation.Token);
    }

    private System.Threading.Tasks.Task<string> Await(string ticket, System.Threading.CancellationToken cancellation)
    {
        return Guid.TryParseExact(ticket, "D", out _) ? Settle(ticket, cancellation) : System.Threading.Tasks.Task.FromResult(ticket);
    }

    private async System.Threading.Tasks.Task<string> Settle(string ticket, System.Threading.CancellationToken cancellation)
    {
        using var watch = cancellation.Register(() => TacHttpTransport.Cancel(ticket));
        try
        {
            var answer = await TacHttpTransport.Answer(ticket).WaitAsync(cancellation);
            cancellation.ThrowIfCancellationRequested();
            return answer;
        }
        finally { TacHttpTransport.Cancel(ticket); }
    }
}
