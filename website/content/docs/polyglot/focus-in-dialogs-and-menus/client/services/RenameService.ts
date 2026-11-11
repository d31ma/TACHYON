@Service
export default class RenameService {
  @Needs
  focus!: Focus;
  editing: boolean = false;
  problem: string = '';

  open(): void {
    this.editing = true;
    this.focus.trap('dialog');
    this.focus.move('name');
  }

  close(): void {
    this.editing = false;
    this.focus.release();
  }

  @OnEvent
  failed(failed: BrowserCallFailed): void {
    this.problem = failed.message;
  }
}
