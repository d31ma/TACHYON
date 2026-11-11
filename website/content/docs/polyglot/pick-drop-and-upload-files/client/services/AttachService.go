import "fmt"

// @Service
type AttachService struct {
	Name     string
	Digest   string
	Status   string
	Problem  string
	selected string
	// @Needs
	files Files
}

func (service *AttachService) Choose() {
	service.keepProblem(service.files.Pick(".pdf,image/*", false))
}

func (service *AttachService) Upload() {
	url := "https://uploads.example.com/inbox"
	_, problem := service.files.Upload(service.selected, "PUT", url, "content-type: application/octet-stream")
	service.keepProblem(problem)
}

// @OnEvent
func (service *AttachService) Picked(picked FilePicked) {
	service.show(picked.Id, picked.Name, picked.Size, picked.Sha256)
}

// @OnEvent
func (service *AttachService) Dropped(dropped FileDropped) {
	service.show(dropped.Id, dropped.Name, dropped.Size, dropped.Sha256)
}

// @OnEvent
func (service *AttachService) Finished(finished UploadFinished) {
	service.Status = fmt.Sprintf("HTTP %v %s", finished.Status, finished.Error)
}

func (service *AttachService) show(id string, name string, size float64, hash string) {
	service.selected = id
	service.Name = fmt.Sprintf("%s, %v bytes", name, size)
	service.Digest = hash
}

func (service *AttachService) keepProblem(problem error) {
	if problem != nil {
		service.Problem = problem.Error()
	}
}
