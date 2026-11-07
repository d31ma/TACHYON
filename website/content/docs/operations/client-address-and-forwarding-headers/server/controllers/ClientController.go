import "net/http"

// @RequestRoute("/api/client")
// @Public
type ClientController struct {
}

// @GetRoute
func (controller *ClientController) GetClient(request *http.Request) map[string]string {
	return map[string]string{
		"address": request.Header.Get("X-Real-IP"),
		"peer":    request.RemoteAddr,
		"chain":   request.Header.Get("X-Forwarded-For"),
	}
}
