// @Service
type MapService struct {
	Latitude  float64
	Longitude float64
}

// @OnEvent
func (service *MapService) Show(value PlaceFound) {
	service.Latitude = value.Latitude
	service.Longitude = value.Longitude
}
