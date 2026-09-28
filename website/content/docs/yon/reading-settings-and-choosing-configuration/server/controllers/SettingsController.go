import (
	"encoding/json"
	"errors"
	"os"
)

// @RequestRoute("/settings")
// @Public
type SettingsController struct{}

// @GetRoute
func (controller *SettingsController) GetSettings() (Settings, error) {
	var application map[string]any
	parseErr := json.Unmarshal([]byte(os.Getenv("YON_APPLICATION_JSON")), &application)
	catalog, catalogErr := BehaviorsProperties[CatalogSetup](application)
	endpoint, endpointErr := BehaviorsValues[Endpoint](application)
	profile := controller.text(endpoint, "profile")
	options, optionsErr := controller.profileOptions(application, profile)
	contextErr := controller.openAndClose(BehaviorsCreate[CatalogSetup](options))
	settings := Settings{Profile: profile, PageSize: int(controller.number(catalog, "pageSize"))}
	return settings, errors.Join(parseErr, catalogErr, endpointErr, optionsErr, contextErr)
}

func (controller *SettingsController) profileOptions(application map[string]any, profile string) (*BehaviorOptions, error) {
	options := NewBehaviorOptions("public-catalog", NewBehaviorSignal())
	options.Profiles = append(options.Profiles, profile)
	return options.WithApplication(application)
}

func (controller *SettingsController) openAndClose(context *BehaviorContext, failure error) error {
	if failure != nil {
		return failure
	}
	context.Dispose()
	return nil
}

func (controller *SettingsController) text(value BehaviorValue, key string) string {
	fields, _ := value.(map[string]any)
	text, _ := fields[key].(string)
	return text
}

func (controller *SettingsController) number(value BehaviorValue, key string) float64 {
	fields, _ := value.(map[string]any)
	number, _ := fields[key].(float64)
	return number
}
