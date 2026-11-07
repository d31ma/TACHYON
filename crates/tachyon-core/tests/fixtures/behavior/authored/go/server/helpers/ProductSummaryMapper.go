type ProductSummaryMapper interface {
	// @Convert({"fields":[{"source":"detail.id","target":"productId"},{"source":"name","target":"title"},{"source":"missing.deep","target":"absent"},{"target":"kind","constant":"product"}]})
	ToSummary(product BehaviorValue) (BehaviorValue, error)
}
