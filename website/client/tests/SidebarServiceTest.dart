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

  List<Object?> currentPaths(List<Map<String, Object?>> sections) => [
        for (final section in sections)
          for (final link in section['links'] as List)
            if ((link as Map)['current'] == true) link['path'],
      ];

  List<Object?> openNames(List<Map<String, Object?>> sections) => [
        for (final section in sections)
          if (section['open'] == true) section['name'],
      ];

  String headings(List<Map<String, Object?>> sections) => [
        for (final section in sections)
          if (section['heading'] != '') section['heading'],
      ].join(',');

  String joined(Iterable<Object?> items, String key) =>
      items.map((item) => (item as Map)[key]).join(',');

  Future<void> testSectionsOpenTheOneHoldingThePage() async {
    final sections = SidebarService('/docs/features/hosts', docs, features).sections();
    Assert.equals(['/docs/features/hosts'], currentPaths(sections), 'exactly one link is current');
    Assert.equals(['Native'], openNames(sections), 'the section holding it is open');
  }

  Future<void> testGuidesAndFeaturesEachGetOneHeading() async {
    final sections = SidebarService('/docs/introduction', docs, features).sections();
    Assert.equals('Guides,Features', headings(sections), 'one Guides and one Features heading');
  }

  Future<void> testAnOrphanPageOpensTheGuide() async {
    final sections = SidebarService('/somewhere-else', docs, features).sections();
    Assert.equals([sections.first['name']], openNames(sections), 'the guide opens alone');
  }

  Future<void> testNavigationOrdersGuidesThenNonEmptyGroups() async {
    Assert.equals('/docs/introduction,/docs/routing', joined(NavigationUtils.guides(docs), 'path'), 'guides in order');
    Assert.equals('routing,native', joined(NavigationUtils.groups(features), 'slug'), 'an empty group is dropped');
    final spaced = NavigationUtils.groups({
      'groups': [' Tac views! '],
      'features': [
        {'id': 'views', 'title': 'Views', 'group': ' Tac views! '},
      ],
    });
    Assert.equals('tac-views', spaced.single['slug'], 'slugs are URL-safe');
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
    Assert.equals('get-started,pages', joined(NavigationUtils.guideGroups(grouped), 'slug'), 'empty docs groups are dropped');
    final sections = SidebarService('/docs/routing', grouped, features).sections();
    Assert.equals('Get started,Pages,Routing,Native', joined(sections, 'name'), 'docs groups, then feature groups');
    Assert.equals('/docs/routing', joined(sections[1]['links'] as List, 'path'), 'an unknown topic is dropped');
    Assert.equals(['Pages'], openNames(sections), 'the group holding the page opens');
  }
}
