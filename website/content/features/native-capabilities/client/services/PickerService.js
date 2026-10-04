@Service
export default class PickerService {
  /** @type {TacFilePicker} */
  @Needs
  filePicker;

  /** @type {string} */
  status = 'Ready';

  /** @returns {Promise<void>} */
  async choose() {
    const result = await this.filePicker.open({ multiple: false, types: ['text/plain'] });
    this.status = result.status;
    await this._readSelected(result);
  }

  /** @param {TacFilePickerResult} result @returns {Promise<void>} */
  async _readSelected(result) {
    if (result.status !== 'selected') return;
    const handle = String(result.handle);
    const content = await this.filePicker.readText(handle);
    this.status = content.status;
    await this.filePicker.release(handle);
  }
}
