package tac

type Files struct{}

func (Files) Pick(accept string, multiple bool) error {
	return Web.Call("files", "pick", []any{accept, multiple}, nil)
}

func (Files) Read(id string, offset int, count int) error {
	return Web.Call("files", "read", []any{id, offset, count}, nil)
}

func (Files) Release(id string) error {
	return Web.Call("files", "release", []any{id}, nil)
}

func (Files) Upload(id string, method string, url string, headers string) (string, error) {
	upload := ""
	err := Web.Call("files", "upload", []any{id, method, url, headers}, &upload)
	return upload, err
}

func (Files) Cancel(upload string) error {
	return Web.Call("files", "cancel", []any{upload}, nil)
}
