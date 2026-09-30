@Service
export default class ReportService {
  @Needs
  windowControls!: TacWindowControls;
  problem = '';

  minimize(): void {
    this.keepProblem(() => this.windowControls.minimize());
  }

  resize(): void {
    this.keepProblem(() => this.windowControls.resize(1280, 800));
  }

  rename(): void {
    this.keepProblem(() => this.windowControls.setTitle('Quarterly report'));
  }

  private keepProblem(call: () => void): void {
    try {
      call();
    } catch (refusal) {
      this.problem = (refusal as Error).message;
    }
  }
}
