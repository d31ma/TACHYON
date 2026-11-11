import type { Problem } from '/models/Problem.ts'

@Service
export default class OrderService {
  @Needs
  readonly orderClient!: OrderClient;
  status = 0
  title = ''
  mistakes = 0

  async send() {
    await this.orderClient.send({ name: '', level: 12 }).catch((error: unknown) => this.showProblem(error))
  }

  private showProblem(error: unknown): void {
    const problem = Behaviors.errorBody<Problem>(error) ?? { title: '', status: 0, errors: [] }
    this.status = Behaviors.errorStatus(error)
    this.title = problem.title
    this.mistakes = problem.errors.length
  }
}
