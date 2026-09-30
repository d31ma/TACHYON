// @Service
type CartService struct {
	Total                     int64
	applicationEventPublisher ApplicationEventPublisher
}

func (cart *CartService) Add() {
	cart.Total++
	cart.applicationEventPublisher.PublishEvent(CartTotal{Total: cart.Total}, true)
}
