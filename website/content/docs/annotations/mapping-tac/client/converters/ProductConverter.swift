@Converter
protocol ProductConverter {
    @Convert({"fields":[{"source":"detail.id","target":"productId"},{"source":"name","target":"title"},{"source":"missing.deep","target":"absent"},{"target":"kind","constant":"product"}]})
    func toSummary(_ product: BehaviorJSON) async throws -> BehaviorJSON
}
