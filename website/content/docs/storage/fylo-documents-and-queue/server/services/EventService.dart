@Service
class EventService {
  static const String _collection = 'hd-events';
  static const String _kind = 'kind';
  static const String _count = 'count';
  static const int _pageSize = 2;
  static const String _topic = 'app.render';
  static const String _group = 'renderers';

  @Needs
  final DataSource dataSource;

  Future<Map<String, Object?>> report() async {
    await dataSource.createCollection(_collection);
    final saved = await dataSource.insert(_collection, _deploys()) as Map<String, Object?>;
    final id = saved['id'] as String;
    await dataSource.insert(_collection, _deploys());
    await dataSource.insert(_collection, _deploys());
    final found = await dataSource.findById(_collection, id) as Map<String, Object?>;
    final patched = await dataSource.patch(_collection, id, _change()) as Map<String, Object?>;
    final scrolled = await _countPages();
    final job = await _runJob();
    await dataSource.deleteById(_collection, id);
    final gone = await dataSource.findById(_collection, id) == null;
    final dropped = await dataSource.dropCollection(_collection);
    return {'found': found[_kind], 'patched': patched[_count], ...scrolled, 'job': job, 'gone': gone, 'dropped': dropped};
  }

  Map<String, Object?> _deploys() => {_kind: 'deploy'};

  Map<String, Object?> _change() => {_count: 7};

  Future<Map<String, int>> _countPages() async {
    var pages = 0;
    var seen = 0;
    String? cursor;
    var more = true;
    while (more) {
      final window = await dataSource.scroll(_collection, _deploys(), cursor, _pageSize) as Map<String, Object?>;
      pages += 1;
      seen += (window['content'] as List).length;
      cursor = window['nextCursor'] as String?;
      more = cursor != null;
    }
    return {'pages': pages, 'seen': seen};
  }

  Future<Object?> _runJob() async {
    await dataSource.publish(_topic, {'app': 'web'}, 'render:web');
    final deliveries = await dataSource.claim(_topic, _group, 1, 30000) as List;
    final delivery = deliveries.first as Map<String, Object?>;
    await dataSource.ack(_topic, _group, delivery);
    return delivery['payload'];
  }
}
