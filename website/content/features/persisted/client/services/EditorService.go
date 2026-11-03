// @Service
type EditorService struct {
	// @SessionState
	Draft string
	// @LocalState
	Theme string
}

// @OnStart
func (editor *EditorService) Start() {
	editor.Theme = "light"
}
