package pricing

func WithTax(cents int) int {
	return cents + cents/10
}
