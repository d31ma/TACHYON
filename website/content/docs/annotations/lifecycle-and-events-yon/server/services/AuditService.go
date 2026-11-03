// @Service
type AuditService struct {
	log     []string
	settled int64
}

// @OnEvent({"id":"audit-log"})
// @Order(-10)
func (audit *AuditService) Record(order PlacedOrder) {
	audit.log = append(audit.log, order.Status)
}

// @OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})
// @Order(10)
func (audit *AuditService) Settle(order PlacedOrder) SettledOrder {
	return SettledOrder{Total: order.Total}
}

// @OnEvent
func (audit *AuditService) SaveSettled(order SettledOrder) {
	audit.settled = order.Total
}
