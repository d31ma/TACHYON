@Service
export default class RenameService {
  /** @type {Focus} */
  @Needs
  focus;
  editing = false;
  problem = '';

  open() {
    this.editing = true;
    this.focus.trap('dialog');
    this.focus.move('name');
  }

  close() {
    this.editing = false;
    this.focus.release();
  }

  /** @param {BrowserCallFailed} failed */
  @OnEvent
  failed(failed) {
    this.problem = failed.message;
  }
}
