@Converter
class ProductConverter {
  @Needs
  behaviors!: BehaviorContext;
  @Convert({"fields":[{"source":"detail.id","target":"productId"},{"source":"name","target":"title"},{"source":"missing.deep","target":"absent"},{"target":"kind","constant":"product"}]})
  async toSummary(product: BehaviorJsonValue): Promise<BehaviorJsonValue> {}
}
