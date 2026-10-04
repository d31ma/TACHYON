@Converter
protocol ProductConverter {
    @Convert({"fields":[{"target":"kind","constant":"product"}]})
    func map(_ product: Product) throws -> Summary

    func copy(_ product: Product) throws -> Product
}
