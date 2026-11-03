@Service
export default class ReportService {
  /** @type {TacWindowControls} */
  @Needs
  windowControls;

  minimize() {
    this.windowControls.minimize();
  }
}
