package tac

type Blobs struct{}

func (Blobs) SaveHeld(fileId string, key string) error {
	return Web.Call("blobs", "saveHeld", []any{fileId, key}, nil)
}

func (Blobs) Download(url string, key string, headers string) error {
	return Web.Call("blobs", "download", []any{url, key, headers}, nil)
}

func (Blobs) List(prefix string) error {
	return Web.Call("blobs", "list", []any{prefix}, nil)
}

func (Blobs) Read(key string, offset int, count int) error {
	return Web.Call("blobs", "read", []any{key, offset, count}, nil)
}

func (Blobs) Remove(key string) error {
	return Web.Call("blobs", "remove", []any{key}, nil)
}

func (Blobs) Url(key string) (string, error) {
	url := ""
	err := Web.Call("blobs", "url", []any{key}, &url)
	return url, err
}

func (Blobs) Open(key string) error {
	return Web.Call("blobs", "open", []any{key}, nil)
}

func (Blobs) Close(key string) error {
	return Web.Call("blobs", "close", []any{key}, nil)
}

func (Blobs) Upload(key string, method string, url string, headers string) (string, error) {
	upload := ""
	err := Web.Call("blobs", "upload", []any{key, method, url, headers}, &upload)
	return upload, err
}

func (Blobs) Cancel(upload string) error {
	return Web.Call("blobs", "cancel", []any{upload}, nil)
}
