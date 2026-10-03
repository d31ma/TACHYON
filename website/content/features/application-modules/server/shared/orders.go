package main

type OrderService struct{}

func (OrderService) Place(item string) string {
	return item
}
