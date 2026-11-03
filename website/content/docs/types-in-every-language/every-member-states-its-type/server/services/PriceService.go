// @Service
type PriceService struct{}

func (service *PriceService) PriceFor(amount int, count int) Price {
	const currency = "EUR"
	return Price{Amount: amount * count, Currency: currency}
}
