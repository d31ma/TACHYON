// @Service
type RuntimeNoteService struct {
	Runtime string
}

// @OnStart
func (note *RuntimeNoteService) Start() {
	note.Runtime = "Go, compiled to WebAssembly"
}

func (note *RuntimeNoteService) ShowsNotes(development bool) bool {
	return development
}
