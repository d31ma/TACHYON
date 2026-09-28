package main

type OrderService struct{}

func (OrderService) Place(item string) map[string]string {
	return map[string]string{"placed": item}
}
