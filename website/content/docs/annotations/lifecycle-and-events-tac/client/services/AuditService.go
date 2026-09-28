type AuditService struct {
	Records int64
	Settled int64
	Audited int64
}

// @OnEvent({"id":"audit-log","replay":true})
// @Order(-10)
func (audit *AuditService) Record(order PlacedOrder) {
	audit.Records++
}

// @OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})
// @Order(10)
func (audit *AuditService) Settle(order PlacedOrder) SettledOrder {
	return SettledOrder{Total: order.Total}
}

// @OnEvent
func (audit *AuditService) SaveSettled(order SettledOrder) {
	audit.Settled = order.Total
}

// @OnEvent
// @InBackground
func (audit *AuditService) Audit(order SettledOrder) {
	audit.Audited = order.Total
}
