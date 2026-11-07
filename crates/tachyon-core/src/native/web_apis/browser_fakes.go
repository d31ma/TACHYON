package tac

type BrowserFakes struct{}

func (BrowserFakes) Answer(api string, call string, value any) {
	Web.fake(api+"."+call, value)
}
