// @Service
type CardService struct {
	Name   string
	Status string
}

// @OnStart
func (card *CardService) Activate() {
	card.Name = "Anvil"
	card.Status = "active"
}
