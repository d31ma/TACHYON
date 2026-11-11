@Service
export default class AttachService {
  @Needs
  files!: TacFiles;

  selected = '';
  name = '';
  digest = '';
  status = '';
  problem = '';

  choose(): void {
    this.files.pick('.pdf,image/*', false);
  }

  upload(): void {
    this.files.upload(this.selected, 'PUT', 'https://uploads.example.com/inbox', 'content-type: application/octet-stream');
  }

  @OnEvent
  picked(picked: FilePicked): void {
    this.name = `${picked.name}, ${picked.size} bytes`;
    this.digest = picked.sha256;
    this.selected = picked.id;
  }

  @OnEvent
  finished(finished: UploadFinished): void {
    this.status = `HTTP ${finished.status} ${finished.error}`;
  }
}
