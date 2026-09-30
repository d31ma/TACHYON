// @Service
type TicketService struct {
	Ticket   string
	IssuedAt float64
	// @Needs
	clock Clock
	// @Needs
	secureRandom SecureRandom
}

func (service *TicketService) Issue() {
	service.Ticket, _ = service.secureRandom.UUID()
	service.IssuedAt, _ = service.clock.NowMs()
}
