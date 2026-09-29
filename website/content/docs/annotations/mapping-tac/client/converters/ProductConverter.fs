[<Converter>]
type ProductConverter =
    [<Convert({"fields":[{"source":"detail.id","target":"productId"},{"source":"name","target":"title"},{"source":"missing.deep","target":"absent"},{"target":"kind","constant":"product"}]})>]
    abstract toSummary: product: BehaviorJSON -> BehaviorJSON
