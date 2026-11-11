final class Files {
  const Files();

  void pick(String accept, bool multiple) =>
      TacWebHost.call('files', 'pick', [accept, multiple]);

  void read(String id, int offset, int count) =>
      TacWebHost.call('files', 'read', [id, offset, count]);

  void release(String id) => TacWebHost.call('files', 'release', [id]);

  String upload(String id, String method, String url, String headers) =>
      TacWebHost.call('files', 'upload', [id, method, url, headers]) as String;

  void cancel(String upload) => TacWebHost.call('files', 'cancel', [upload]);
}
