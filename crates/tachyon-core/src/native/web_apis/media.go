package tac

type Media struct{}

func (Media) Supported(feature string) (bool, error) {
	value := false
	err := Web.Call("media", "supported", []any{feature}, &value)
	return value, err
}

func (Media) Devices(request string) error {
	return Web.Call("media", "devices", []any{request}, nil)
}

func (Media) ChooseSpeaker(device string) error {
	return Web.Call("media", "chooseSpeaker", []any{device}, nil)
}

func (Media) CaptureMicrophone(track string, device string, processing bool) error {
	return Web.Call("media", "captureMicrophone", []any{track, device, processing}, nil)
}

func (Media) CaptureCamera(track string, device string, quality string) error {
	return Web.Call("media", "captureCamera", []any{track, device, quality}, nil)
}

func (Media) CaptureScreen(track string, withAudio bool) error {
	return Web.Call("media", "captureScreen", []any{track, withAudio}, nil)
}

func (Media) StopTrack(track string) error {
	return Web.Call("media", "stopTrack", []any{track}, nil)
}

func (Media) Mute(track string, muted bool) error {
	return Web.Call("media", "mute", []any{track, muted}, nil)
}
