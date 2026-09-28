import (
	"slices"
	"strings"
)

// @Service
// @NeedsAll
type CatalogService struct {
	Source string
	// @Required
	Features Catalogue
}

// @OnStart
func (catalog *CatalogService) Start() {
	catalog.Source = "website/content/features/page.json"
}

func (catalog *CatalogService) List() []FeatureGroup {
	groups := catalog.groupsWith(catalog.featuresByGroup())
	return slices.DeleteFunc(groups, catalog.hasNoFeatures)
}

func (catalog *CatalogService) Count() int {
	return catalog.countFeatures(catalog.List())
}

func (catalog *CatalogService) Contents() []TopicContents {
	return catalog.contentsOf(catalog.List())
}

func (catalog *CatalogService) featuresByGroup() map[string][]Feature {
	byGroup := map[string][]Feature{}
	for _, feature := range catalog.Features.Features {
		feature.Path = "/docs/features/" + feature.ID
		byGroup[feature.Group] = append(byGroup[feature.Group], feature)
	}
	return byGroup
}

func (catalog *CatalogService) groupsWith(byGroup map[string][]Feature) []FeatureGroup {
	groups := []FeatureGroup{}
	for _, name := range catalog.Features.Groups {
		groups = append(groups, FeatureGroup{Name: name, Slug: catalog.slug(name), Features: byGroup[name]})
	}
	return groups
}

func (catalog *CatalogService) hasNoFeatures(group FeatureGroup) bool {
	return len(group.Features) == 0
}

func (catalog *CatalogService) slug(name string) string {
	return strings.Join(strings.FieldsFunc(strings.ToLower(name), catalog.isSeparator), "-")
}

func (catalog *CatalogService) isSeparator(character rune) bool {
	return !strings.ContainsRune("abcdefghijklmnopqrstuvwxyz0123456789", character)
}

func (catalog *CatalogService) countFeatures(groups []FeatureGroup) int {
	total := 0
	for _, group := range groups {
		total += len(group.Features)
	}
	return total
}

func (catalog *CatalogService) contentsOf(groups []FeatureGroup) []TopicContents {
	contents := []TopicContents{}
	for _, group := range groups {
		contents = append(contents, TopicContents{Heading: group.Name, Anchor: "#" + group.Slug})
	}
	return contents
}
