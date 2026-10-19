// @Helper
type CatalogEchoHelper struct{}

func (helper *CatalogEchoHelper) Exchange(request *BehaviorHttpRequest, execution *BehaviorExecution) (BehaviorValue, error) {
	var body BehaviorValue
	if request.HasBody {
		body = request.Body
	}
	return map[string]any{"method": request.Method, "path": request.Path, "query": request.Query, "headers": request.Headers, "body": body}, nil
}
