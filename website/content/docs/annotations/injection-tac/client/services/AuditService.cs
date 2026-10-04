[Service]
public sealed class AuditService {
    private readonly CartHelper _ledger;

    public AuditService() {
        _ledger = new CartHelper();
    }

    public int Lines() {
        return _ledger.Count();
    }
}
