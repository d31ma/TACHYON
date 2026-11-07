// @Service
type EventService struct {
	// @Needs
	dataSource DataSource
}

func (service *EventService) Report() (EventReport, error) {
	collection := service.collection()
	err := service.dataSource.CreateCollection(collection)
	id, _ := service.dataSource.Insert(collection, service.deploys())
	service.dataSource.Insert(collection, service.deploys())
	service.dataSource.Insert(collection, service.deploys())
	report := service.readAndPatch(id)
	report.Pages, report.Seen = service.countPages()
	report.Job = service.runJob()
	service.dataSource.DeleteById(collection, id)
	exists, _ := service.dataSource.FindById(collection, id, nil)
	report.Gone = !exists
	report.Dropped, _ = service.dataSource.DropCollection(collection)
	return report, err
}

func (service *EventService) collection() string { return "hd-events" }

func (service *EventService) deploys() Event { return Event{Kind: "deploy"} }

func (service *EventService) topic() string { return "app.render" }

func (service *EventService) group() string { return "renderers" }

func (service *EventService) readAndPatch(id string) EventReport {
	found := Event{}
	service.dataSource.FindById(service.collection(), id, &found)
	patched := Event{}
	service.dataSource.Patch(service.collection(), id, Event{Kind: found.Kind, Count: 7}, &patched)
	return EventReport{Found: found.Kind, Patched: patched.Count}
}

func (service *EventService) countPages() (int, int) {
	pages, seen, cursor, more := 0, 0, "", true
	for more {
		window, _ := service.dataSource.Scroll(service.collection(), map[string]any{"kind": service.deploys().Kind}, cursor, 2)
		events := []Event{}
		window.Decode(&events)
		pages++
		seen += len(events)
		cursor = window.NextCursor
		more = cursor != ""
	}
	return pages, seen
}

func (service *EventService) runJob() Job {
	service.dataSource.Publish(service.topic(), Job{App: "web"}, "render:web")
	deliveries, _ := service.dataSource.Claim(service.topic(), service.group(), 1, 30000)
	job := Job{}
	deliveries[0].Decode(&job)
	service.dataSource.Ack(service.topic(), service.group(), deliveries[0])
	return job
}
