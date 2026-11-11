// @Schedule({"fixedDelaySeconds":300})
type Handler struct {
	// @Needs
	ledger *LedgerService
}

func (worker *Handler) POST() int {
	return worker.ledger.Total()
}
