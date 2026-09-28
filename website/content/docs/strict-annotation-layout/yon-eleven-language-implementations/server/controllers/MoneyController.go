// @RequestRoute("/money")
// @Public
type MoneyController struct {
	// @Needs
	money *MoneyService
}

// @GetRoute
func (controller *MoneyController) GetMoney() Money {
	return Money{Amount: controller.money.Amount()}
}
