// @HTTPClient
type ApplicationClient interface {
	// @GetRequest("/manifest.webmanifest")
	Load() (BehaviorValue, error)
}
