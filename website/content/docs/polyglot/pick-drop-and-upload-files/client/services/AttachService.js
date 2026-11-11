@Service
export default class AttachService {
  @Needs
  files;

  selected = '';
  name = '';
  digest = '';
  status = '';
  problem = '';

  choose() {
    this.files.pick('.pdf,image/*', false);
  }

  upload() {
    this.files.upload(this.selected, 'PUT', 'https://uploads.example.com/inbox', 'content-type: application/octet-stream');
  }

  /** @param {FilePicked} picked */
  @OnEvent
  picked(picked) {
    this.name = `${picked.name}, ${picked.size} bytes`;
    this.digest = picked.sha256;
    this.selected = picked.id;
  }

  /** @param {UploadFinished} finished */
  @OnEvent
  finished(finished) {
    this.status = `HTTP ${finished.status} ${finished.error}`;
  }
}
