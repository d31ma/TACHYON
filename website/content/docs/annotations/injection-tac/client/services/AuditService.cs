[Service]
public sealed class AuditService {
    [Needs]
    private readonly CartDelegate _ledger;

    public int Lines() {
        return _ledger.Count();
    }
}
