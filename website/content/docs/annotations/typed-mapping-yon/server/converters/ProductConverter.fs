open Models
[<Converter>]
type ProductConverter =
    [<Convert({"fields":[{"target":"kind","constant":"product"}]})>]
    abstract map: product: Product -> Summary
