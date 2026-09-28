// @HTTPClient
type CatalogClient struct {
	behaviors *BehaviorContext
}

// @OnStart
func (client *CatalogClient) Start() error {
	options := NewBehaviorOptions("catalog", NewBehaviorSignal())
	context, err := BehaviorsCreate[CatalogSetup](options)
	client.behaviors = context
	return err
}

// @GetRequest("/products")
func (client *CatalogClient) All(
	// @RequestParam("page")
	page BehaviorValue,
) (BehaviorValue, error) {
}

// @PostRequest("/products")
func (client *CatalogClient) Add(
	// @RequestBody
	product BehaviorValue,
) (BehaviorValue, error) {
}

// @GetRequest("/products")
func (client *CatalogClient) First() (BehaviorValue, error) {
	query := map[string]any{"page": 1}
	return map[string]any{"query": query}, nil
}
