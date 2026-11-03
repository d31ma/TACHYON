import "fmt"

type PriceUtils struct{}

func (PriceUtils) Label(cents int) string {
	return "$" + PriceUtils{}.dollars(cents)
}

func (PriceUtils) dollars(cents int) string {
	return fmt.Sprintf("%.2f", float64(cents)/100)
}
