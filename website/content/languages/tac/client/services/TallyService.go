// @Service
type TallyService struct {
	Count   int
	Runtime string
}

// @OnStart
func (tally *TallyService) Start() {
	tally.Count = 6
	tally.Runtime = "Go, compiled to WebAssembly"
}

func (tally *TallyService) Doubled() int {
	return tally.Count * 2
}
