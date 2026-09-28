@Service
export class OrderService {
  /** @type {OrderStore} */
  _repository = new OrderStore()

  /** @returns {Order[]} */
  list() {
    return this._repository.all()
  }
}
