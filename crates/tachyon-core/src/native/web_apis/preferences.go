package tac

type Preferences struct{}

func (preferences Preferences) Scheme() (string, error) {
	return preferences.text("scheme")
}

func (preferences Preferences) Motion() (string, error) {
	return preferences.text("motion")
}

func (preferences Preferences) Language() (string, error) {
	return preferences.text("language")
}

func (Preferences) text(call string) (string, error) {
	value := ""
	err := Web.Call("preferences", call, nil, &value)
	return value, err
}
