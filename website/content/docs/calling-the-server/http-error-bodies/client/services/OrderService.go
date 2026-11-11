// @Service
type OrderService struct {
	// @Needs
	orderClient *OrderClient
	Status      int
	Title       string
	Mistakes    int
}

func (service *OrderService) Send() error {
	_, err := service.orderClient.Send(OrderInput{Name: "", Level: 12})
	var problem Problem
	BehaviorErrorBody(err, &problem)
	service.Status = BehaviorErrorStatus(err)
	service.Title = problem.Title
	service.Mistakes = len(problem.Errors)
	return nil
}
