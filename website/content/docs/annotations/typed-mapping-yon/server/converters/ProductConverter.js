@Converter
class ProductConverter {
/** @param {Product} product
 * @returns {Summary} */
@Convert({"fields":[{"target":"kind","constant":"product"}]})
async map(product) {}
}
