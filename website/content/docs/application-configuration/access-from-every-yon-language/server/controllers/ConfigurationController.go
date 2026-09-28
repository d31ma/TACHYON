import (
	"encoding/json"
	"errors"
	"os"
)

// @RequestRoute("/configuration")
// @Public
type ConfigurationController struct{}

// @GetRoute
func (controller *ConfigurationController) GetConfiguration() (Configuration, error) {
	var application map[string]any
	parseErr := json.Unmarshal([]byte(os.Getenv("YON_APPLICATION_JSON")), &application)
	settings, settingsErr := BehaviorsProperties[CatalogSettings](application)
	endpoint, endpointErr := BehaviorsValues[Endpoint](application)
	configuration := Configuration{PageSize: controller.number(settings, "pageSize"), Port: controller.number(endpoint, "port")}
	return configuration, errors.Join(parseErr, settingsErr, endpointErr)
}

func (controller *ConfigurationController) number(value BehaviorValue, key string) int {
	fields, _ := value.(map[string]any)
	number, _ := fields[key].(float64)
	return int(number)
}
