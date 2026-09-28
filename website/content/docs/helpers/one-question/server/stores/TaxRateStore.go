// @Store
type TaxRateStore struct{}

func (store *TaxRateStore) Rate() int {
	return 2
}
