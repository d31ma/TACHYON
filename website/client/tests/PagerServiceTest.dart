@Test(SiteTestSetup, {"partition":"tests"})
class PagerServiceTest {
  final docs = <String, Object?>{
    'order': ['introduction', 'routing'],
    'topics': {
      'introduction': {'title': 'Introduction'},
      'routing': {'title': 'Routing'},
    },
  };
  final features = <String, Object?>{
    'groups': ['Routing', 'Native', 'Tooling'],
    'features': [
      {'id': 'files', 'title': 'File routes', 'group': 'Routing'},
      {'id': 'hosts', 'title': 'Native hosts', 'group': 'Native'},
    ],
  };

  Future<void> testPreviousAndNextFollowTheReadingOrder() async {
    final last = PagerService('/docs/routing', docs, features);
    if (last.previous()?['path'] != '/docs/introduction') throw StateError('previous is the guide before');
    if (last.next()?['path'] != '/docs/features/files') throw StateError('next crosses into the features');
    if (PagerService('/docs/features/files', docs, features).previous()?['path'] != '/docs/routing') {
      throw StateError('previous and next are inverses');
    }
  }

  Future<void> testTheEndsAndAnUnknownPageHaveNoNeighbour() async {
    if (PagerService('/docs/introduction', docs, features).previous() != null) throw StateError('the first page has no previous');
    if (PagerService('/docs/features/hosts', docs, features).next() != null) throw StateError('the last page has no next');
    final stray = PagerService('/nowhere', docs, features);
    if (stray.previous() != null || stray.next() != null) throw StateError('a page outside the order has none');
  }
}
