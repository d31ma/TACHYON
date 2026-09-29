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
    Assert.equals('/docs/introduction', last.previous()?['path'], 'previous is the guide before');
    Assert.equals('/docs/features/files', last.next()?['path'], 'next crosses into the features');
    final first = PagerService('/docs/features/files', docs, features);
    Assert.equals('/docs/routing', first.previous()?['path'], 'previous and next are inverses');
  }

  Future<void> testTheEndsAndAnUnknownPageHaveNoNeighbour() async {
    Assert.equals(null, PagerService('/docs/introduction', docs, features).previous(), 'the first page has no previous');
    Assert.equals(null, PagerService('/docs/features/hosts', docs, features).next(), 'the last page has no next');
    final stray = PagerService('/nowhere', docs, features);
    Assert.equals(null, stray.previous(), 'a page outside the order has no previous');
    Assert.equals(null, stray.next(), 'a page outside the order has no next');
  }
}
