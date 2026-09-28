class NavigationUtils {
  static List<Map<String, Object?>> guides(Map<String, Object?>? documents) {
    final topics = _mapAt(documents, _topicsKey());
    final slugs = _listAt(documents, _orderKey());
    return _guideLinks(slugs, topics);
  }

  static List<Map<String, Object?>> guideGroups(Map<String, Object?>? documents) {
    final guideLinks = guides(documents);
    final links = _linksByPath(guideLinks);
    final groups = _listAt(documents, _groupsKey()).whereType<Map>().toList();
    final named = _nonEmpty(_namedGuideGroups(groups, links));
    return _groupsOrGuide(groups, named, guideLinks);
  }

  static List<Map<String, Object?>> groups(Map<String, Object?>? catalogue) {
    final features = _listAt(catalogue, _featuresKey()).whereType<Map>().toList();
    final names = _listAt(catalogue, _groupsKey());
    return _nonEmpty(_featureGroups(names, features));
  }

  static String _slugOf(String value) {
    final dashed = value.toLowerCase().replaceAll(RegExp('[^a-z0-9]+'), '-');
    return dashed.replaceAll(RegExp(r'^-|-$'), '');
  }

  static Map _mapAt(Map? source, String key) {
    return (source?[key] as Map?) ?? const {};
  }

  static List _listAt(Map? source, String key) {
    return (source?[key] as List?) ?? const [];
  }

  static List<Map<String, Object?>> _guideLinks(List slugs, Map topics) {
    return [for (final slug in slugs) _guideLink(slug, topics)];
  }

  static Map<String, Object?> _guideLink(Object? slug, Map topics) {
    final title = _titleOr(topics[slug], slug);
    return {_titleKey(): '$title', _pathKey(): _guidePath(slug)};
  }

  static Object? _titleOr(Object? topic, Object? fallback) {
    return (topic as Map?)?[_titleKey()] ?? fallback;
  }

  static Map<Object?, Map<String, Object?>> _linksByPath(List<Map<String, Object?>> links) {
    return {for (final link in links) link[_pathKey()]: link};
  }

  static List<Map<String, Object?>> _namedGuideGroups(
    List<Map> groups,
    Map<Object?, Map<String, Object?>> links,
  ) {
    return [for (final group in groups) _guideGroup(group, links)];
  }

  static Map<String, Object?> _guideGroup(Map group, Map<Object?, Map<String, Object?>> links) {
    final paths = _guidePaths(_listAt(group, _topicsKey()));
    return _group('${group[_titleKey()]}', _linksAt(paths, links));
  }

  static List<String> _guidePaths(List slugs) {
    return [for (final slug in slugs) _guidePath(slug)];
  }

  static List<Map<String, Object?>> _linksAt(
    List<String> paths,
    Map<Object?, Map<String, Object?>> links,
  ) {
    return [for (final path in paths.where(links.containsKey)) links[path]!];
  }

  static List<Map<String, Object?>> _groupsOrGuide(
    List<Map> groups,
    List<Map<String, Object?>> named,
    List<Map<String, Object?>> guideLinks,
  ) {
    if (groups.isEmpty) return [_group('Guide', guideLinks)];
    return named;
  }

  static List<Map<String, Object?>> _featureGroups(List names, List<Map> features) {
    return [for (final name in names) _featureGroup(name, features)];
  }

  static Map<String, Object?> _featureGroup(Object? name, List<Map> features) {
    return _group('$name', _featureLinks(name, features));
  }

  static List<Map<String, Object?>> _featureLinks(Object? name, List<Map> features) {
    final members = features.where((feature) => feature['group'] == name);
    return [for (final feature in members) _featureLink(feature)];
  }

  static Map<String, Object?> _featureLink(Map feature) {
    return {_titleKey(): '${feature[_titleKey()]}', _pathKey(): '/docs/features/${feature['id']}'};
  }

  static Map<String, Object?> _group(String name, List<Map<String, Object?>> links) {
    return {_nameKey(): name, _slugKey(): _slugOf(name), _linksKey(): links};
  }

  static List<Map<String, Object?>> _nonEmpty(List<Map<String, Object?>> groups) {
    return groups.where(_hasLinks).toList();
  }

  static bool _hasLinks(Map<String, Object?> group) {
    return (group[_linksKey()] as List).isNotEmpty;
  }

  static String _guidePath(Object? slug) => '/docs/$slug';

  static String _titleKey() => 'title';

  static String _pathKey() => 'path';

  static String _nameKey() => 'name';

  static String _slugKey() => 'slug';

  static String _linksKey() => 'links';

  static String _groupsKey() => 'groups';

  static String _topicsKey() => 'topics';

  static String _orderKey() => 'order';

  static String _featuresKey() => 'features';
}
