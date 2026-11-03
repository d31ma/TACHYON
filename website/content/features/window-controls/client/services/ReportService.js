@Service
export default class ReportService {
  /** @type {TacWindowControls} */
  @Needs
  windowControls;
  problem = '';

  minimize() {
    this._keepProblem(() => this.windowControls.minimize());
  }

  resize() {
    this._keepProblem(() => this.windowControls.resize(1280, 800));
  }

  rename() {
    this._keepProblem(() => this.windowControls.setTitle('Quarterly report'));
  }

  /** @param {() => void} call */
  _keepProblem(call) {
    try {
      call();
    } catch (refusal) {
      this.problem = /** @type {Error} */ (refusal).message;
    }
  }
}
