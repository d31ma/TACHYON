package tac

type Share struct{}

func (Share) Open(title string, text string, url string) error {
	return Web.Call("share", "open", []any{title, text, url}, nil)
}
