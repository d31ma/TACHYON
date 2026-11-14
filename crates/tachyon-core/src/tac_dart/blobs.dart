
final class Blobs {
  const Blobs();

  void saveHeld(String fileId, String key) =>
      TacWebHost.call('blobs', 'saveHeld', [fileId, key]);

  void download(String url, String key, String headers) =>
      TacWebHost.call('blobs', 'download', [url, key, headers]);

  void list(String prefix) => TacWebHost.call('blobs', 'list', [prefix]);

  void read(String key, int offset, int count) =>
      TacWebHost.call('blobs', 'read', [key, offset, count]);

  void remove(String key) => TacWebHost.call('blobs', 'remove', [key]);

  String url(String key) => TacWebHost.call('blobs', 'url', [key]) as String;

  void open(String key) => TacWebHost.call('blobs', 'open', [key]);

  void close(String key) => TacWebHost.call('blobs', 'close', [key]);

  String upload(String key, String method, String url, String headers) =>
      TacWebHost.call('blobs', 'upload', [key, method, url, headers]) as String;

  void cancel(String upload) => TacWebHost.call('blobs', 'cancel', [upload]);
}
