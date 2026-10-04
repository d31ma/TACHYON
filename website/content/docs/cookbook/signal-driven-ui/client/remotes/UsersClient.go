// @HTTPClient
type UsersClient interface {
	// @PostRequest("/users")
	Save(
		// @RequestBody
		name string,
	) (UserSaved, error)
}
