import "strings"

// @Test(SiteTestSetup, {"partition":"tests"})
type CatalogServiceTest struct {
	behaviors *BehaviorContext
}

func (test *CatalogServiceTest) catalogue() Catalogue {
	return Catalogue{
		Groups: []string{"Tac views", "Native", "Tooling"},
		Features: []Feature{
			{ID: "loops", Title: "LOOPS", Summary: "About loops", Group: "Tac views"},
			{ID: "hosts", Title: "HOSTS", Summary: "About hosts", Group: "Native"},
			{ID: "slots", Title: "SLOTS", Summary: "About slots", Group: "Tac views"},
		},
	}
}

func (test *CatalogServiceTest) filtered() *CatalogService {
	return &CatalogService{Features: Catalogue{
		Groups: []string{"Routing", "Browser storage"},
		Features: []Feature{
			{ID: "pages", Title: "Page routing", Summary: "Folders define the pages.", Group: "Routing"},
			{ID: "controllers", Title: "Controller routing", Summary: "Annotations give the paths.", Group: "Routing"},
			{ID: "storage", Title: "Browser storage", Summary: "Folders of records in the browser.", Group: "Browser storage"},
		},
	}}
}

func (test *CatalogServiceTest) slugs(groups []FeatureGroup) string {
	slugs := []string{}
	for _, group := range groups {
		slugs = append(slugs, group.Slug)
	}
	return strings.Join(slugs, ",")
}

func (test *CatalogServiceTest) TestListGroupsEveryFeatureInTheCataloguesOrder() {
	catalog := &CatalogService{Features: test.catalogue()}
	listed := catalog.List()
	Assert.Equals("tac-views,native", test.slugs(listed), "an empty group is dropped, the rest keep their order")
	Assert.Equals(3, catalog.Count(), "count agrees with the groups")
	first := listed[0].Features[0]
	Assert.Equals("/docs/features/loops", first.Path, "a feature links to its page")
	Assert.Equals("About loops", first.Summary)
}

func (test *CatalogServiceTest) TestContentsAnchorEachGroupAndStartNamesTheData() {
	catalog := &CatalogService{Features: test.catalogue()}
	contents := catalog.Contents()
	Assert.Equals(2, len(contents), "one fragment per group")
	Assert.Equals("#tac-views", contents[0].Anchor)
	catalog.Start()
	Assert.Equals("website/content/features/page.json", catalog.Source)
	Assert.Equals(0, (&CatalogService{}).Count(), "no catalogue is no features")
}

func (test *CatalogServiceTest) TestMatchCountFollowsTheFeatureAndGroupFilters() {
	catalog := test.filtered()
	Assert.Equals(3, catalog.MatchCount("", ""), "no filter counts every feature")
	Assert.Equals(2, catalog.MatchCount("", "Routing"), "a group counts only its features")
	Assert.Equals(2, catalog.MatchCount("folders", ""), "a query counts the features it shows")
	Assert.Equals(1, catalog.MatchCount("folders", "Routing"), "the query and the group both apply")
	Assert.Equals(0, catalog.MatchCount("sockets", ""), "no match counts zero")
}

func (test *CatalogServiceTest) TestMatchCountMatchesEveryWordInAnyCase() {
	catalog := test.filtered()
	Assert.Equals(1, catalog.MatchCount("  Folders ROUTING ", ""), "words match in any case, and extra spaces are ignored")
	Assert.Equals(1, catalog.MatchCount("paths", ""), "a word may match the summary")
	Assert.Equals(0, catalog.MatchCount("folders sockets", ""), "every word must match")
}
