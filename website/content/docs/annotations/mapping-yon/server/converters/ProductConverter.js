@Converter
class ProductConverter {
  /** @type {BehaviorContext} */
  @Needs
  behaviors;
  @Convert({"fields":[{"source":"detail.id","target":"productId"},{"source":"name","target":"title"},{"source":"missing.deep","target":"absent"},{"target":"kind","constant":"product"}]})
  /** @param {object} product */
  async toSummary(product) {}
}
