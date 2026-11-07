// @Service
type WriterService struct {
	// @Needs
	pageProgramClient *PageProgramClient
	// @Needs
	programNoteStore *ProgramNoteStore
	Identity         string
	Count            int
	Recalled         string
}

func (WriterService) noteId() string { return "note" }

func (service *WriterService) Inspect() error {
	state, err := service.pageProgramClient.Read()
	service.show(state, err)
	return err
}

func (service *WriterService) Increment() error {
	state, err := service.pageProgramClient.Increment()
	service.show(state, err)
	return err
}

func (service *WriterService) Remember(text string) error {
	_, err := service.programNoteStore.Save(map[string]any{"id": service.noteId(), "text": text})
	return err
}

func (service *WriterService) Recall() error {
	note, err := service.programNoteStore.FindById(service.noteId())
	fields, _ := note.(map[string]any)
	service.Recalled = service.textOf(fields)
	return err
}

func (service *WriterService) textOf(fields map[string]any) string {
	text, _ := fields["text"].(string)
	return text
}

func (service *WriterService) show(state ProgramState, failure error) {
	if failure != nil {
		return
	}
	service.Identity = state.Identity
	service.Count = state.Count
}
