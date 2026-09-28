package tac

type Geolocation struct{}

func (Geolocation) CurrentPosition() error {
	return Web.Call("geolocation", "currentPosition", nil, nil)
}
