// @Delegate
type TaxDelegate struct {
	// @Needs
	taxService *TaxService
}

func (delegate *TaxDelegate) Rate(region string) int {
	return delegate.taxService.Rate(region) * 2
}

func (delegate *TaxDelegate) TotalWith(count int, extra int) int {
	return delegate.taxService.Total(count) + extra
}
