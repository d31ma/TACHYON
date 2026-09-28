@Converter
abstract class ProductConverter {
  @Convert({"fields":[{"source":"detail.id","target":"productId"},{"source":"name","target":"title"},{"source":"missing.deep","target":"absent"},{"target":"kind","constant":"product"}]})
  Object? toSummary(Object? product);
}
