import (
	"encoding/json"
	"sync/atomic"
)

// @Description("Behavior conformance exercise helpers")
type CatalogChecks struct{}

func (CatalogChecks) same(left any, right any) bool {
	leftText, leftError := json.Marshal(left)
	rightText, rightError := json.Marshal(right)
	return leftError == nil && rightError == nil && string(leftText) == string(rightText)
}

func (CatalogChecks) fail(message string) error {
	return &BehaviorError{Code: "adapter", Message: message}
}

func (CatalogChecks) productInput() map[string]any {
	return map[string]any{"title": "é", "quantity": 2}
}

func (CatalogChecks) blankProductInput() map[string]any {
	return map[string]any{"title": "\u00a0"}
}

func (CatalogChecks) memory(context *BehaviorContext) *Memory {
	return context.configuration.Entities["data"].(*Memory)
}

func (checks CatalogChecks) checkValidation() error {
	input := checks.productInput()
	validated, err := BehaviorsValidate[ProductInput](input)
	if err != nil {
		return err
	}
	if !checks.same(validated, input) {
		return checks.fail("validation changed the input")
	}
	checked, err := PureValidator{}.Check(input)
	if err != nil {
		return err
	}
	if !checks.same(checked, input) {
		return checks.fail("the pure validator changed the input")
	}
	_, err = PureValidator{}.Check(checks.blankProductInput())
	if BehaviorErrorCode(err) != "validation" || BehaviorErrorPath(err) != "title.notBlank" {
		return checks.fail("a blank title was accepted")
	}
	return nil
}

func (checks CatalogChecks) checkSettings() error {
	environment := map[string]string{"YON_DATABASE_HOST": "localhost", "YON_DATABASE_PORT": "9000"}
	settings, err := BehaviorsProperties[Settings](environment)
	if err != nil {
		return err
	}
	database := map[string]any{"host": "localhost", "port": 9000}
	nested, err := BehaviorsProperties[Settings](map[string]any{"yon": map[string]any{"database": database}})
	if err != nil {
		return err
	}
	if !checks.same(nested, settings) {
		return checks.fail("nested settings differ from environment settings")
	}
	values := settings.(map[string]any)
	if values["port"] != 9000.0 || values["enabled"] != false {
		return checks.fail("settings were not bound")
	}
	return nil
}

func (checks CatalogChecks) checkApplicationValues() error {
	server := map[string]any{"host": "127.0.0.1", "port": 9000}
	application := map[string]any{"yon": map[string]any{"server": server, "database": map[string]any{"port": 6543}}}
	bound, err := BehaviorsValues[ApplicationValues](application)
	if err != nil {
		return err
	}
	values := bound.(map[string]any)
	if values["database"] != 6543.0 || values["label"] != "primary" {
		return checks.fail("application values were not bound")
	}
	unknown := map[string]any{"secrets": map[string]any{}}
	_, err = BehaviorsValues[ApplicationValues](unknown)
	if BehaviorErrorCode(err) != "configuration-properties" {
		return checks.fail("an unknown section was accepted")
	}
	manifest := map[string]any{"name": "Catalog", "yon": map[string]any{"database": map[string]any{"port": 7000}}}
	manifestValues, err := BehaviorsValues[ApplicationValues](manifest)
	if err != nil {
		return err
	}
	if manifestValues.(map[string]any)["database"] != 7000.0 {
		return checks.fail("manifest values were not bound")
	}
	return nil
}

func (checks CatalogChecks) checkSelectedConfig() error {
	inactive := NewBehaviorOptions("must-not-execute", NewBehaviorSignal())
	_, err := BehaviorsCreate[SelectedConfigSetup](inactive)
	if BehaviorErrorCode(err) != "inactive-configuration" {
		return checks.fail("an inactive configuration was selected")
	}
	application := map[string]any{"yon": map[string]any{"cache": map[string]any{"enabled": true}}}
	selected := NewBehaviorOptions("configured", NewBehaviorSignal())
	selected.Profiles = append(selected.Profiles, "test")
	selected, err = selected.WithApplication(application)
	if err != nil {
		return err
	}
	context, err := BehaviorsCreate[SelectedConfigSetup](selected)
	if err != nil {
		return err
	}
	context.Dispose()
	return nil
}

func (checks CatalogChecks) checkValidationBeforeAuthorization(catalog *CatalogService) error {
	input := checks.productInput()
	denied, err := BehaviorsCreate[ConfigSetup](NewBehaviorOptions("denied", NewBehaviorSignal()))
	if err != nil {
		return err
	}
	deniedCatalog := &CatalogService{behaviors: denied, calls: &atomic.Int64{}}
	_, err = deniedCatalog.Validated(checks.blankProductInput())
	if BehaviorErrorCode(err) != "validation" {
		return checks.fail("validation must run before cache authorization")
	}
	_, err = deniedCatalog.Validated(input)
	if BehaviorErrorCode(err) != "unauthorized" {
		return checks.fail("valid input still needs authorization")
	}
	validated, err := catalog.Validated(input)
	if err != nil {
		return err
	}
	if !checks.same(validated, input) {
		return checks.fail("the cached validation changed the input")
	}
	return nil
}

func (checks CatalogChecks) checkCatalogCache(context *BehaviorContext, catalog *CatalogService, client *CatalogClient) (map[string]any, error) {
	page := map[string]any{"page": 2, "size": 5}
	first, err := catalog.List(page)
	if err != nil {
		return nil, err
	}
	cached, err := catalog.List(page)
	if err != nil {
		return nil, err
	}
	if !checks.same(first, cached) {
		return nil, checks.fail("the second call was not cached")
	}
	if err := context.Evict(catalog, "List", []BehaviorValue{page}); err != nil {
		return nil, err
	}
	fresh, err := catalog.List(page)
	if err != nil {
		return nil, err
	}
	remote, err := client.Remote()
	if err != nil {
		return nil, err
	}
	if _, err := catalog.Refresh(page, "replacement"); err != nil {
		return nil, err
	}
	listed, err := catalog.List(page)
	if err != nil || listed != "replacement" {
		return nil, checks.fail("CachePut did not replace the entry")
	}
	if _, err := catalog.Invalidate(page); err != nil {
		return nil, err
	}
	listed, err = catalog.List(page)
	if err != nil || listed.(map[string]any)["calls"] != 3.0 {
		return nil, checks.fail("CacheEvict did not remove the entry")
	}
	found, err := client.Find(7)
	if err != nil || found != "/products/7" {
		return nil, checks.fail("the path variable did not reach the adapter")
	}
	return map[string]any{"first": first, "cached": cached, "fresh": fresh, "remote": remote}, nil
}

func (checks CatalogChecks) checkRepository(context *BehaviorContext, results map[string]any) error {
	repository, err := BehaviorsRepository[ProductEntity](context)
	if err != nil {
		return err
	}
	saved, err := repository.Save(map[string]any{"id": "one"})
	if err != nil {
		return err
	}
	found, err := repository.Find(map[string]any{})
	if err != nil {
		return err
	}
	deleted, err := repository.Delete("one")
	if err != nil {
		return err
	}
	results["saved"] = saved
	results["found"] = found
	results["deleted"] = deleted
	return nil
}

func (checks CatalogChecks) checkResilience(context *BehaviorContext) error {
	attempts := &atomic.Int64{}
	resilient := &Resilient{behaviors: context, attempts: attempts}
	value, err := resilient.Flaky()
	if err != nil {
		return err
	}
	if value != 3.0 || attempts.Load() != 3 {
		return checks.fail("Retryable did not retry twice")
	}
	bounded, err := resilient.Bounded("ok")
	if err != nil || bounded != "ok" {
		return checks.fail("ConcurrencyLimit changed the value")
	}
	return nil
}

func (checks CatalogChecks) checkTransactions(context *BehaviorContext) error {
	memory := checks.memory(context)
	memory.TakeLog()
	ledger := &Ledger{behaviors: context}
	if _, err := ledger.Post("one"); err != nil {
		return err
	}
	if !checks.same(memory.TakeLog(), []string{"begin", "save@tx-1", "commit:tx-1"}) {
		return checks.fail("a posted entry must commit")
	}
	if _, err := ledger.Refuse("two"); err == nil {
		return checks.fail("a refused entry must fail")
	}
	if !checks.same(memory.TakeLog(), []string{"begin", "save@tx-1", "rollback:tx-1"}) {
		return checks.fail("a refused entry must roll back")
	}
	return nil
}

func (checks CatalogChecks) checkQueries(context *BehaviorContext) error {
	products := &ProductQueriesStore{behaviors: context}
	rows, err := products.FindByStatus("active")
	if err != nil {
		return err
	}
	if found, isList := rows.([]any); !isList || len(found) != 1 {
		return checks.fail("FindByStatus did not return one row")
	}
	count, err := products.CountByStatus("active")
	if err != nil || count != 1.0 {
		return checks.fail("CountByStatus did not count one row")
	}
	exists, err := products.ExistsById("one")
	if err != nil || exists != true {
		return checks.fail("ExistsById did not find the row")
	}
	selected, err := products.Selected("active", map[string]any{"size": 1})
	if err != nil {
		return err
	}
	if !checks.same(selected, rows) {
		return checks.fail("Selected did not return the same rows")
	}
	return checks.checkRepository(products)
}

func (checks CatalogChecks) checkRepository(products *ProductQueriesStore) error {
	found, err := products.FindById("one")
	if row, isRow := found.(map[string]any); err != nil || !isRow || row["id"] != "one" {
		return checks.fail("FindById did not return the row")
	}
	all, err := products.FindAll(map[string]any{})
	if rows, isList := all.([]any); err != nil || !isList || len(rows) != 1 {
		return checks.fail("FindAll did not return one row")
	}
	saved, err := products.Save(map[string]any{"id": "one"})
	if row, isRow := saved.(map[string]any); err != nil || !isRow || row["operation"] != "save" {
		return checks.fail("Save did not save the row")
	}
	deleted, err := products.DeleteById("one")
	if err != nil || deleted != 1.0 {
		return checks.fail("DeleteById did not delete one row")
	}
	removed, err := products.DeleteByStatus("gone")
	if err != nil || removed != 1.0 {
		return checks.fail("DeleteByStatus did not delete one row")
	}
	return nil
}

func (checks CatalogChecks) checkMapper(context *BehaviorContext) error {
	summaries := &ProductSummaryMapper{behaviors: context}
	product := map[string]any{"detail": map[string]any{"id": 7}, "name": "Anvil", "extra": "dropped"}
	summary, err := summaries.ToSummary(product)
	if err != nil {
		return err
	}
	expected := map[string]any{"productId": 7, "title": "Anvil", "absent": nil, "kind": "product"}
	if !checks.same(summary, expected) {
		return checks.fail("Mapping produced another summary")
	}
	_, err = summaries.ToSummary([]any{})
	if BehaviorErrorCode(err) != "invalid" {
		return checks.fail("Mapping accepted a list")
	}
	return nil
}

func (checks CatalogChecks) Exercise() (BehaviorValue, error) {
	if err := checks.checkValidation(); err != nil {
		return nil, err
	}
	if err := checks.checkSettings(); err != nil {
		return nil, err
	}
	if err := checks.checkApplicationValues(); err != nil {
		return nil, err
	}
	if err := checks.checkSelectedConfig(); err != nil {
		return nil, err
	}
	signal := NewBehaviorSignal()
	context, err := BehaviorsCreate[ConfigSetup](NewBehaviorOptions("tenant-a", signal))
	if err != nil {
		return nil, err
	}
	catalog := &CatalogService{behaviors: context, calls: &atomic.Int64{}}
	client := &CatalogClient{behaviors: context}
	if err := checks.checkValidationBeforeAuthorization(catalog); err != nil {
		return nil, err
	}
	results, err := checks.checkCatalogCache(context, catalog, client)
	if err != nil {
		return nil, err
	}
	if err := checks.checkRepository(context, results); err != nil {
		return nil, err
	}
	if err := checks.checkResilience(context); err != nil {
		return nil, err
	}
	if err := checks.checkTransactions(context); err != nil {
		return nil, err
	}
	if err := checks.checkQueries(context); err != nil {
		return nil, err
	}
	if err := checks.checkMapper(context); err != nil {
		return nil, err
	}
	signal.Abort()
	if _, err := client.Remote(); BehaviorErrorCode(err) != "aborted" {
		return nil, checks.fail("an aborted signal must stop the exchange")
	}
	context.Dispose()
	return results, nil
}

func (checks CatalogChecks) Answer() any {
	application := map[string]any{"yon": map[string]any{"server": map[string]any{"host": "127.0.0.1", "port": 9000}}}
	bridged, err := BehaviorsValues[ApplicationValues](application)
	if err != nil || bridged.(map[string]any)["database"] != 5432.0 {
		return map[string]any{"error": "json-bridge"}
	}
	value, err := checks.Exercise()
	if err != nil {
		return map[string]any{"error": err.Error()}
	}
	return value
}
