// @Service
type TaxService struct{}

func (service *TaxService) Rate(region string) int {
	return 2
}

func (service *TaxService) Total(count int) int {
	return count * 10
}
