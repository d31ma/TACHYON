import (
	"io"
	"net/http"
	"strings"
)

type Middleware struct{}

func (Middleware) Before(request *http.Request) *http.Response {
	if strings.HasPrefix(request.URL.Path, "/private") && request.Header.Get("Authorization") != "let-me-in" {
		header := http.Header{"Content-Type": {"application/json"}}
		body := io.NopCloser(strings.NewReader(`{"error":"Unauthorized"}`))
		return &http.Response{StatusCode: http.StatusUnauthorized, Header: header, Body: body}
	}
	return nil
}

func (Middleware) After(request *http.Request) *http.Response {
	header := http.Header{"X-Served-By": {"tachyon"}}
	return &http.Response{StatusCode: http.StatusNoContent, Header: header}
}
