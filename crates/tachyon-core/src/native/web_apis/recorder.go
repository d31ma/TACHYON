package tac

type Recorder struct{}

func (Recorder) Supported() (bool, error) {
	value := false
	err := Web.Call("recorder", "supported", nil, &value)
	return value, err
}

func (Recorder) Start(recording string, audioTracks string, videoTrack string, fileName string, format string) error {
	return Web.Call("recorder", "start", []any{recording, audioTracks, videoTrack, fileName, format}, nil)
}

func (Recorder) AddAudio(recording string, track string) error {
	return Web.Call("recorder", "addAudio", []any{recording, track}, nil)
}

func (Recorder) RemoveAudio(recording string, track string) error {
	return Web.Call("recorder", "removeAudio", []any{recording, track}, nil)
}

func (Recorder) ShowVideo(recording string, track string) error {
	return Web.Call("recorder", "showVideo", []any{recording, track}, nil)
}

func (Recorder) Pause(recording string) error {
	return Web.Call("recorder", "pause", []any{recording}, nil)
}

func (Recorder) Resume(recording string) error {
	return Web.Call("recorder", "resume", []any{recording}, nil)
}

func (Recorder) Stop(recording string) error {
	return Web.Call("recorder", "stop", []any{recording}, nil)
}
