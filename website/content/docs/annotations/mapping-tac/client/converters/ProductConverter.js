@Converter
export default class ProductConverter {
  /**
   * @param {BehaviorJsonValue} product
   * @returns {Promise<BehaviorJsonObject | void>}
   */
  @Convert({"fields":[{"source":"detail.id","target":"productId"},{"source":"name","target":"title"},{"source":"missing.deep","target":"absent"},{"target":"kind","constant":"product"}]})
  async toSummary(product) {}
}
