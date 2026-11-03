// @Service
type GuideService struct {
	Title string
	Pages int64
}

// @OnStart
func (service *GuideService) Start() {
	guide := Guide{Title: "Models hold types", Pages: 3}
	service.Title = guide.Title
	service.Pages = guide.Pages
}
