import (
	"errors"
	"strings"
)

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

func (test *CatalogServiceTest) TestListGroupsEveryFeatureInTheCataloguesOrder() error {
	catalog := &CatalogService{Features: test.catalogue()}
	listed := catalog.List()
	slugs := []string{}
	for _, group := range listed {
		slugs = append(slugs, group.Slug)
	}
	if strings.Join(slugs, ",") != "tac-views,native" {
		return errors.New("an empty group is dropped, the rest keep their order")
	}
	if catalog.Count() != 3 {
		return errors.New("count agrees with the groups")
	}
	first := listed[0].Features[0]
	if first.Path != "/docs/features/loops" || first.Summary != "About loops" {
		return errors.New("a feature links to its page")
	}
	return nil
}

func (test *CatalogServiceTest) TestContentsAnchorEachGroupAndStartNamesTheData() error {
	catalog := &CatalogService{Features: test.catalogue()}
	contents := catalog.Contents()
	if len(contents) != 2 || contents[0].Anchor != "#tac-views" {
		return errors.New("one fragment per group")
	}
	catalog.Start()
	if catalog.Source != "website/content/features/page.json" {
		return errors.New(catalog.Source)
	}
	if (&CatalogService{}).Count() != 0 {
		return errors.New("no catalogue is no features")
	}
	return nil
}
