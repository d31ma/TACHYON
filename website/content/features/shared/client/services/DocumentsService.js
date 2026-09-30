@Service
@NeedsAll
export default class DocumentsService {
  /** @type {string[]} */
  order

  /** @returns {number} */
  guides() {
    return this.order.length
  }
}
