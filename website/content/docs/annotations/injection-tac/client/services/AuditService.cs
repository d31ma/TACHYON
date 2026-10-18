[Service]
public sealed class AuditService {
    [Needs]
    private readonly CartHelper _ledger;

    public int Lines() {
        return _ledger.Count();
    }
}
