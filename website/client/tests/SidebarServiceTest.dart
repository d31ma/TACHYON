@Test(SiteTestSetup, {"partition":"tests"})
class SidebarServiceTest {
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

  Future<void> testSectionsOpenTheOneHoldingThePage() async {
    final sections = SidebarService('/docs/features/hosts', docs, features).sections();
    final current = [
      for (final section in sections)
        for (final link in section['links'] as List) if ((link as Map)['current'] == true) link,
    ];
    if (current.length != 1 || current.single['path'] != '/docs/features/hosts') throw StateError('exactly one link is current');
    final open = sections.where((section) => section['open'] == true).toList();
    if (open.length != 1 || open.single['name'] != 'Native') throw StateError('the section holding it is open');
  }

  Future<void> testGuidesAndFeaturesEachGetOneHeading() async {
    final headings = [
      for (final section in SidebarService('/docs/introduction', docs, features).sections())
        if (section['heading'] != '') section['heading'],
    ];
    if (headings.join(',') != 'Guides,Features') throw StateError('one Guides and one Features heading');
  }

  Future<void> testAnOrphanPageOpensTheGuide() async {
    final sections = SidebarService('/somewhere-else', docs, features).sections();
    if (sections.first['open'] != true || sections.skip(1).any((section) => section['open'] == true)) {
      throw StateError('the guide opens alone');
    }
  }

  Future<void> testNavigationOrdersGuidesThenNonEmptyGroups() async {
    final guides = NavigationUtils.guides(docs);
    if (guides.map((guide) => guide['path']).join(',') != '/docs/introduction,/docs/routing') throw StateError('guides in order');
    final groups = NavigationUtils.groups(features);
    if (groups.map((group) => group['slug']).join(',') != 'routing,native') throw StateError('an empty group is dropped');
    final spaced = NavigationUtils.groups({
      'groups': [' Tac views! '],
      'features': [
        {'id': 'views', 'title': 'Views', 'group': ' Tac views! '},
      ],
    });
    if (spaced.single['slug'] != 'tac-views') throw StateError('slugs are URL-safe');
  }

  Future<void> testGuideGroupsFollowTheDocsGroups() async {
    final grouped = <String, Object?>{
      ...docs,
      'groups': [
        {'title': 'Get started', 'topics': ['introduction']},
        {'title': 'Pages', 'topics': ['routing', 'missing']},
        {'title': 'Empty', 'topics': []},
      ],
    };
    final guideGroups = NavigationUtils.guideGroups(grouped);
    if (guideGroups.map((group) => group['slug']).join(',') != 'get-started,pages') throw StateError('empty docs groups are dropped');
    final sections = SidebarService('/docs/routing', grouped, features).sections();
    if (sections.map((section) => section['name']).join(',') != 'Get started,Pages,Routing,Native') throw StateError('docs groups, then feature groups');
    final pages = sections[1]['links'] as List;
    if (pages.map((link) => (link as Map)['path']).join(',') != '/docs/routing') throw StateError('an unknown topic is dropped');
    if (sections[1]['open'] != true || sections[0]['open'] == true) throw StateError('the group holding the page opens');
  }
}
