import "errors"

// @Service
type SettingsService struct {
	// @Needs
	applicationClient *ApplicationClient
	Api               string
}

func (settings *SettingsService) Load() error {
	application, loadErr := settings.applicationClient.Load()
	values, valuesErr := BehaviorsValues[PublicSettings](application)
	settings.Api = settings.text(values, "api")
	return errors.Join(loadErr, valuesErr)
}

func (settings *SettingsService) text(value BehaviorValue, key string) string {
	fields, _ := value.(map[string]any)
	text, _ := fields[key].(string)
	return text
}
