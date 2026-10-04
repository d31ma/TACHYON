// @HTTPClient("/api")
type OverrideClient interface {
	// @GetRequest("/never")
	Read() (Greeting, error)
}
