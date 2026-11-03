import "net/http"

// @IdentityProvider
type AppIdentityProvider struct {
}

func (provider *AppIdentityProvider) Identify(request *http.Request) *Identity {
	return provider.find(request.Header.Get("Authorization"))
}

func (provider *AppIdentityProvider) find(token string) *Identity {
	return provider.sessions()[token]
}

func (provider *AppIdentityProvider) sessions() map[string]*Identity {
	return map[string]*Identity{
		"ada-token": {User: "ada", Roles: []string{"Admin"}},
		"bob-token": {User: "bob", Roles: []string{}},
	}
}
