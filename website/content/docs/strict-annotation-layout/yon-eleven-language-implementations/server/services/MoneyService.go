// @Service
type MoneyService struct{}

func (service *MoneyService) Amount() int64 {
	return 42
}
