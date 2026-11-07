import "strings"

// @Setup
type PageProgramSetup struct {
	identity string
	count    int
}

func (PageProgramSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {
	identity, err := (SecureRandom{}).UUID()
	if err != nil {
		return nil, err
	}
	configuration := NewBehaviorConfiguration()
	configuration.Http["pageprogram"] = &PageProgramSetup{identity: identity}
	return configuration, nil
}

func (adapter *PageProgramSetup) Exchange(request *BehaviorHttpRequest, execution *BehaviorExecution) (BehaviorValue, error) {
	if request.Method == "POST" && strings.HasSuffix(request.Path, "/increment") {
		adapter.count++
	}
	return ProgramState{Identity: adapter.identity, Count: adapter.count}, nil
}
