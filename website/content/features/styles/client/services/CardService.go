// @Service
type CardService struct {
	Name string
}

// @OnStart
func (card *CardService) Start() {
	card.Name = "Anvil"
}
