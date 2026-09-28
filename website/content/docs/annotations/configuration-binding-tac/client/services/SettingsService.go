import "errors"

type SettingsService struct {
	Title    string
	PageSize int64
}

// @OnStart
func (settings *SettingsService) Start() error {
	published := settings.publicManifest()
	catalog, catalogErr := BehaviorsProperties[CatalogSetup](published)
	values, valuesErr := BehaviorsValues[PublicValues](published)
	options, optionsErr := settings.publicOptions(published)
	contextErr := settings.openAndClose(BehaviorsCreate[CatalogSetup](options))
	settings.Title = settings.text(values, settings.titleKey())
	settings.PageSize = settings.number(catalog, "pageSize")
	return errors.Join(catalogErr, valuesErr, optionsErr, contextErr)
}

func (settings *SettingsService) publicOptions(published map[string]any) (*BehaviorOptions, error) {
	options := NewBehaviorOptions("public-catalog", NewBehaviorSignal())
	options.Profiles = append(options.Profiles, "public")
	return options.WithApplication(published)
}

func (settings *SettingsService) openAndClose(context *BehaviorContext, failure error) error {
	if failure != nil {
		return failure
	}
	context.Dispose()
	return nil
}

func (settings *SettingsService) text(value BehaviorValue, key string) string {
	fields, _ := value.(map[string]any)
	text, _ := fields[key].(string)
	return text
}

func (settings *SettingsService) number(value BehaviorValue, key string) int64 {
	fields, _ := value.(map[string]any)
	number, _ := fields[key].(float64)
	return int64(number)
}

func (SettingsService) titleKey() string { return "title" }

func (settings *SettingsService) publicManifest() map[string]any {
	catalog := map[string]any{
		settings.titleKey(): "Catalog",
		"page":              map[string]any{"size": 20},
		"cache":             map[string]any{"enabled": true},
	}
	return map[string]any{"tac": map[string]any{"catalog": catalog}}
}
