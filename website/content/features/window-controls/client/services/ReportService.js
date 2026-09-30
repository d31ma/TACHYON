@Service
export default class ReportService {
  /** @type {TacWindowControls} */
  @Needs
  windowControls;
  problem = '';

  minimize() {
    this.#keepProblem(() => this.windowControls.minimize());
  }

  resize() {
    this.#keepProblem(() => this.windowControls.resize(1280, 800));
  }

  rename() {
    this.#keepProblem(() => this.windowControls.setTitle('Quarterly report'));
  }

  /** @param {() => void} call */
  #keepProblem(call) {
    try {
      call();
    } catch (refusal) {
      this.problem = refusal.message;
    }
  }
}
