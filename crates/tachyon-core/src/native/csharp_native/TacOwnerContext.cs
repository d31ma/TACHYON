internal sealed class TacOwnerContext : System.Threading.SynchronizationContext
{
    private readonly object ownerLock;

    private TacOwnerContext(object ownerLock) { this.ownerLock = ownerLock; }

    internal static T Run<T>(object ownerLock, Func<T> work)
    {
        var previous = Current;
        SetSynchronizationContext(new TacOwnerContext(ownerLock));
        try { return work(); }
        finally { SetSynchronizationContext(previous); }
    }

    public override System.Threading.SynchronizationContext CreateCopy() => new TacOwnerContext(ownerLock);

    public override void Post(System.Threading.SendOrPostCallback callback, object state) =>
        System.Threading.ThreadPool.QueueUserWorkItem(_ => RunLocked(callback, state));

    public override void Send(System.Threading.SendOrPostCallback callback, object state) => RunLocked(callback, state);

    private void RunLocked(System.Threading.SendOrPostCallback callback, object state)
    {
        lock (ownerLock)
        {
            var previous = Current;
            SetSynchronizationContext(this);
            try { callback(state); }
            finally { SetSynchronizationContext(previous); }
        }
    }
}
