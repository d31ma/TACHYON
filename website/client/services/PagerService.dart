@Service
@NeedsAll
class PagerService {
  late final String path;
  late final Map<String, Object?> documents;
  late final Map<String, Object?> features;

  Map<String, Object?>? previous() => _pageNextToCurrent(-1);

  Map<String, Object?>? next() => _pageNextToCurrent(1);

  Map<String, Object?>? _pageNextToCurrent(int step) {
    final pages = _readingOrder();
    return _pageNextTo(pages, _currentIndex(pages), step);
  }

  List<Map<String, Object?>> _readingOrder() {
    final pages = NavigationUtils.guides(documents);
    for (final group in NavigationUtils.groups(features)) {
      pages.addAll((group['links'] as List).cast<Map<String, Object?>>());
    }
    return pages;
  }

  int _currentIndex(List<Map<String, Object?>> pages) => pages.map(_pathOf).toList().indexOf(path);

  Object? _pathOf(Map<String, Object?> page) => page['path'];

  Map<String, Object?>? _pageNextTo(List<Map<String, Object?>> pages, int index, int step) {
    if (index < 0) return null;
    final withEmptyEnds = <Map<String, Object?>?>[null, ...pages, null];
    return withEmptyEnds[index + 1 + step];
  }
}
