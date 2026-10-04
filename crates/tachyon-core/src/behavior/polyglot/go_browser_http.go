package main

var behaviorBrowserCodes = []string{
	"HTTP_REQUEST", "HTTP_ORIGIN", "HTTP_HEADER", "HTTP_QUERY", "HTTP_BODY", "HTTP_LIMIT", "HTTP_RESPONSE",
	"HTTP_STATUS", "HTTP_CAPACITY", "HTTP_NETWORK", "HTTP_MISSING", "CANCELLED", "DEADLINE",
}

const behaviorBrowserFailed = "The browser HTTP request failed."

func behaviorDefaultExchange(request *BehaviorHttpRequest, operation *behaviorOperation, publicRead bool) (BehaviorValue, error) {
	text, err := behaviorEncode(behaviorBrowserRequest(request, operation, publicRead), 65536)
	if err != nil {
		return nil, err
	}
	if err := operation.execution.Check(); err != nil {
		return nil, err
	}
	answer := behaviorBrowserHost()(text)
	if err := operation.execution.Check(); err != nil {
		return nil, err
	}
	return behaviorBrowserAnswer(answer)
}

func behaviorBrowserRequest(request *BehaviorHttpRequest, operation *behaviorOperation, publicRead bool) map[string]any {
	fields := map[string]any{
		"method":           request.Method,
		"path":             request.Path,
		"query":            request.Query,
		"headers":          request.Headers,
		"redirect":         request.Redirect,
		"credentials":      request.Credentials,
		"deadline":         operation.execution.Deadline,
		"maxResponseBytes": request.MaxResponseBytes,
		"publicRead":       publicRead,
		"revalidate":       operation.revalidate,
	}
	if request.HasBody {
		fields["body"] = request.Body
	}
	return fields
}

func behaviorBrowserAnswer(text string) (BehaviorValue, error) {
	parsed, err := behaviorParseJSON(text)
	object, isObject := parsed.(map[string]any)
	if err != nil || !isObject {
		return nil, behaviorError("HTTP_RESPONSE", behaviorBrowserFailed)
	}
	if failure, failed := object["error"].(map[string]any); failed {
		return nil, behaviorError(behaviorBrowserCode(failure["code"]), behaviorBrowserFailed)
	}
	value, present := object["value"]
	if !present {
		return nil, behaviorError("HTTP_RESPONSE", behaviorBrowserFailed)
	}
	return behaviorChecked(value, 1048576)
}

func behaviorBrowserCode(code any) string {
	for _, known := range behaviorBrowserCodes {
		if code == known {
			return known
		}
	}
	return "HTTP_NETWORK"
}

func BehaviorsAnswer[T any](value BehaviorValue, failure error) (T, error) {
	var answer T
	if failure != nil {
		return answer, failure
	}
	text, err := json.Marshal(value)
	_, isObject := value.(map[string]any)
	wanted := reflect.TypeOf(&answer).Elem().Kind() != reflect.Struct || isObject
	if err != nil || !wanted || json.Unmarshal(text, &answer) != nil {
		return answer, behaviorError("HTTP_RESPONSE", "The server answer does not have the shape of the model.")
	}
	return answer, nil
}
