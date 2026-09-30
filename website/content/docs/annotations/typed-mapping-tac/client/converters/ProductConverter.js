@Converter
export default class ProductConverter {
/** @param {Product} product
 * @returns {Summary} */
@Convert({"fields":[{"target":"kind","constant":"product"}]})
async map(product) {}
/** @param {Product} product
 * @returns {Product} */
async copy(product) {}
}
