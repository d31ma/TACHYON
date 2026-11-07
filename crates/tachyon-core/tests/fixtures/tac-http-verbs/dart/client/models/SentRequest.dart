class SentRequest {
  final String method;
  final String path;
  final String header;
  final String item;

  const SentRequest(this.method, this.path, this.header, this.item);
}
