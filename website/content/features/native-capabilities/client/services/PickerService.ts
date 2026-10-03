@Service
export default class PickerService {
  @Needs
  filePicker!: TacFilePicker;

  status: string = 'Ready';

  async choose(): Promise<void> {
    const result = await this.filePicker.open({ multiple: false, types: ['text/plain'] });
    this.status = result.status;
    await this.readSelected(result);
  }

  private async readSelected(result: TacFilePickerResult): Promise<void> {
    if (result.status !== 'selected') return;
    const handle: string = String(result.handle);
    const content: TacFilePickerResult = await this.filePicker.readText(handle);
    this.status = content.status;
    await this.filePicker.release(handle);
  }
}
