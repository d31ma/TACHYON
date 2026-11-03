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

func (catalog *CatalogService) MatchCount(query string, group string) int {
	words := strings.FieldsFunc(strings.ToLower(query), catalog.isSpace)
	inGroup := slices.DeleteFunc(catalog.listedFeatures(), func(feature Feature) bool { return catalog.isOtherGroup(feature.Group, group) })
	return len(slices.DeleteFunc(inGroup, func(feature Feature) bool { return !catalog.hasEveryWord(feature, words) }))
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

func (catalog *CatalogService) listedFeatures() []Feature {
	features := []Feature{}
	for _, group := range catalog.List() {
		features = append(features, group.Features...)
	}
	return features
}

func (catalog *CatalogService) isOtherGroup(name string, group string) bool {
	return group != "" && group != name
}

func (catalog *CatalogService) space() rune {
	return ' '
}

func (catalog *CatalogService) isSpace(character rune) bool {
	return character == catalog.space()
}

func (catalog *CatalogService) hasEveryWord(feature Feature, words []string) bool {
	text := strings.ToLower(feature.Title + string(catalog.space()) + feature.Summary)
	return !slices.ContainsFunc(words, func(word string) bool { return !strings.Contains(text, word) })
}

func (catalog *CatalogService) contentsOf(groups []FeatureGroup) []TopicContents {
	contents := []TopicContents{}
	for _, group := range groups {
		contents = append(contents, TopicContents{Heading: group.Name, Anchor: "#" + group.Slug})
	}
	return contents
}
