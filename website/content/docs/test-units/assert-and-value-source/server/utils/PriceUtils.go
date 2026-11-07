import (
	"errors"
	"math"
)

type PriceUtils struct{}

func (PriceUtils) WithTax(cents int64) (int64, error) {
	if cents < 0 {
		return 0, errors.New("A price is never negative.")
	}
	return int64(math.Round(float64(cents) * 1.2)), nil
}
