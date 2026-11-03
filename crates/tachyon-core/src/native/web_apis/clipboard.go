package tac

type Clipboard struct{}

func (Clipboard) WriteText(text string) error {
	return Web.Call("clipboard", "writeText", []any{text}, nil)
}

func (Clipboard) ReadText() error {
	return Web.Call("clipboard", "readText", nil, nil)
}
