@Service
@NeedsAll
class SidebarService {
  static const _headingKey = 'heading';
  static const _linksKey = 'links';
  static const _pathKey = 'path';

  late final String path;
  late final Map<String, Object?> documents;
  late final Map<String, Object?> features;

  List<Map<String, Object?>> sections() {
    final sections = [
      ..._headed(NavigationUtils.guideGroups(documents), 'Guides'),
      ..._headed(NavigationUtils.groups(features), 'Features'),
    ];
    return _opened(sections, _openFlags(sections));
  }

  List<Map<String, Object?>> _headed(List<Map<String, Object?>> groups, String heading) => [
        ..._first(groups, heading),
        ..._rest(groups),
      ];

  List<Map<String, Object?>> _first(List<Map<String, Object?>> groups, String heading) => [
        for (final group in groups.take(1)) {...group, _headingKey: heading},
      ];

  List<Map<String, Object?>> _rest(List<Map<String, Object?>> groups) => [
        for (final group in groups.skip(1)) {...group, _headingKey: ''},
      ];

  List<bool> _openFlags(List<Map<String, Object?>> sections) {
    final holding = sections.map(_holdsCurrentPage).toList();
    if (holding.contains(true)) return holding;
    return [true, ...List.filled(sections.length - 1, false)];
  }

  bool _holdsCurrentPage(Map<String, Object?> section) =>
      (section[_linksKey] as List).map(_pathOf).contains(path);

  Object? _pathOf(Object? link) => (link as Map)[_pathKey];

  List<Map<String, Object?>> _opened(List<Map<String, Object?>> sections, List<bool> open) => [
        for (final (index, section) in sections.indexed)
          {...section, _linksKey: _markCurrentLink(section[_linksKey] as List), 'open': open[index]},
      ];

  List<Map<String, Object?>> _markCurrentLink(List links) =>
      links.cast<Map<String, Object?>>().map(_marked).toList();

  Map<String, Object?> _marked(Map<String, Object?> link) => {...link, 'current': link[_pathKey] == path};
}
