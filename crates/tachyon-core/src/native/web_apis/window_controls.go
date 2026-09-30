package tac

type WindowControls struct{}

func (WindowControls) Granted(control string) (bool, error) {
	value := false
	err := Web.Call("windowControls", "granted", []any{control}, &value)
	return value, err
}

func (WindowControls) Minimize() error {
	return Web.Call("windowControls", "minimize", nil, nil)
}

func (WindowControls) Maximize() error {
	return Web.Call("windowControls", "maximize", nil, nil)
}

func (WindowControls) Fullscreen() error {
	return Web.Call("windowControls", "fullscreen", nil, nil)
}

func (WindowControls) Close() error {
	return Web.Call("windowControls", "close", nil, nil)
}

func (WindowControls) Resize(width int, height int) error {
	return Web.Call("windowControls", "resize", []any{width, height}, nil)
}

func (WindowControls) SetTitle(text string) error {
	return Web.Call("windowControls", "setTitle", []any{text}, nil)
}
