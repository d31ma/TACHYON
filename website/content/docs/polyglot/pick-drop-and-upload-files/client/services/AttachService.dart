@Service()
class AttachService {
  @Needs()
  final Files files;
  String selected = '';
  String name = '';
  String digest = '';
  String status = '';
  String problem = '';

  void choose() => files.pick('.pdf,image/*', false);

  void upload() => files.upload(selected, 'PUT', 'https://uploads.example.com/inbox',
      'content-type: application/octet-stream');

  @OnEvent
  void picked(FilePicked picked) {
    name = '${picked.name}, ${picked.size} bytes';
    digest = picked.sha256;
    selected = picked.id;
  }

  @OnEvent
  void finished(UploadFinished finished) {
    status = 'HTTP ${finished.status} ${finished.error}';
  }
}
