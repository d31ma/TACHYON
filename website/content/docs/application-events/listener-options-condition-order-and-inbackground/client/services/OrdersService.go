// @Service
type OrdersService struct {
	Log                       []string
	Settled                   int64
	Audited                   int64
	applicationEventPublisher ApplicationEventPublisher
}

func (orders *OrdersService) Place(total int64) {
	order := PlacedOrder{Status: "paid", Total: total}
	orders.applicationEventPublisher.PublishEvent(order, false)
}

// @OnEvent({"id":"orders-log"})
// @Order(-10)
func (orders *OrdersService) Record(order PlacedOrder) {
	orders.Log = append(orders.Log, order.Status)
}

// @OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})
func (orders *OrdersService) SettleLargeOrder(order PlacedOrder) SettledOrder {
	orders.Settled++
	return SettledOrder{Total: order.Total}
}

// @OnEvent
// @InBackground
func (orders *OrdersService) Audit(order SettledOrder) {
	orders.Audited = order.Total
}
