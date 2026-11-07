// @Converter
type ProductConverter interface {
	// @Convert({"fields":[{"target":"kind","constant":"product"}]})
	Map(product Product) (Summary, error)
}
