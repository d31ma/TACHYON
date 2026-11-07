import "iter"

// @RequestRoute("/ticks")
// @Public
type TicksController struct{}

// @StreamRoute
func (controller *TicksController) StreamTicks() iter.Seq[Tick] {
	return controller.countdown(3)
}

func (controller *TicksController) countdown(start int) iter.Seq[Tick] {
	return func(yield func(Tick) bool) {
		for level := start; level >= 1; level-- {
			if !yield(Tick{Sku: "ANVIL-1", Level: level}) {
				return
			}
		}
	}
}
