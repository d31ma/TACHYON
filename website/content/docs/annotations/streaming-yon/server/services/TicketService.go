// @Service
type TicketService struct{}

func (service *TicketService) Levels() []int {
	return []int{1, 2, 3}
}
