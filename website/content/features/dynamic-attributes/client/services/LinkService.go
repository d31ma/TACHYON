// @Service
type LinkService struct {
	Topic   Topic
	State   string
	Message string
}

// @OnStart
func (link *LinkService) Start() {
	link.Topic = Topic{Slug: "routing", Title: "Routing"}
	link.State = "ready"
	link.Message = "Everything is up to date."
}
