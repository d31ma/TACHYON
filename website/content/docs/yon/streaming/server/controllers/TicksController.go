import "iter"

// @RequestRoute("/ticks")
// @Public
type TicksController struct{}

// @StreamRoute
func (controller *TicksController) StreamTicks() iter.Seq[Tick] {
	return controller.ticksUpTo(3)
}

func (controller *TicksController) ticksUpTo(last int) iter.Seq[Tick] {
	return func(yield func(Tick) bool) {
		for level := 1; level <= last; level++ {
			if !yield(Tick{Sku: "ANVIL-1", Level: level}) {
				return
			}
		}
	}
}
