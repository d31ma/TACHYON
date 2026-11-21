import (
	"errors"
	"net/http"
)

// @IdentityProvider
type AppIdentityProvider struct {
}

func (provider *AppIdentityProvider) Identify(request *http.Request) (*Identity, error) {
	return provider.find(request.Header.Get("Authorization"))
}

func (provider *AppIdentityProvider) find(token string) (*Identity, error) {
	if token == "broken-token" {
		return nil, errors.New("the session store is down")
	}
	return provider.sessions()[token], nil
}

func (provider *AppIdentityProvider) sessions() map[string]*Identity {
	return map[string]*Identity{
		"ada-token": {User: "ada", Roles: []string{"Admin"}},
		"bob-token": {User: "bob", Roles: []string{}},
	}
}
