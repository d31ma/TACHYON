@Service
export default class OrderService {
  /** @type {OrderClient} */
  @Needs
  orderClient;
  status = 0
  title = ''
  mistakes = 0

  async send() {
    await this.orderClient.send({ name: '', level: 12 }).catch((error) => this._showProblem(error))
  }

  /**
   * @param {unknown} error
   * @returns {void}
   */
  _showProblem(error) {
    /** @type {Problem} */
    const problem = Behaviors.errorBody(error) ?? { title: '', status: 0, errors: [] }
    this.status = Behaviors.errorStatus(error)
    this.title = problem.title
    this.mistakes = problem.errors.length
  }
}
