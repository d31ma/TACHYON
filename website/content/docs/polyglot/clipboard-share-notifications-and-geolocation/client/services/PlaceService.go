import "fmt"

// @Service
type PlaceService struct {
	Place   string
	Problem string
	// @Needs
	geolocation Geolocation
	// @Needs
	clipboard Clipboard
}

func (service *PlaceService) Locate() {
	service.keepProblem(service.geolocation.CurrentPosition())
}

func (service *PlaceService) Copy() {
	service.keepProblem(service.clipboard.WriteText(service.Place))
}

// @OnEvent
func (service *PlaceService) Found(found PositionFound) {
	service.Place = fmt.Sprintf("%v, %v", found.Latitude, found.Longitude)
}

// @OnEvent
func (service *PlaceService) Failed(failed BrowserCallFailed) {
	service.Problem = failed.Message
}

func (service *PlaceService) keepProblem(problem error) {
	if problem != nil {
		service.Problem = problem.Error()
	}
}
