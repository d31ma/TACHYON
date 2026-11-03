// @Service
type EditorService struct {
	// @SessionState
	Draft string
	// @LocalState
	Theme string
}

// @OnStart
func (editor *EditorService) Start() {
	editor.Theme = editor.startingTheme()
}

func (editor *EditorService) startingTheme() string {
	if editor.Theme == "" {
		return "calm"
	}
	return editor.Theme
}
