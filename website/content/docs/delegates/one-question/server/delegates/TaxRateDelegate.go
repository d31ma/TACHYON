// @Delegate
type TaxRateDelegate struct{}

func (helper *TaxRateDelegate) Rate() int {
	return 2
}
