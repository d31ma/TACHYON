import models.Tick

@RequestRoute("/ticks")
@Public
class TicksController {
    @StreamRoute
    Iterator<Tick> streamTicks() {
        return countdown(3)
    }

    private Iterator<Tick> countdown(int start) {
        return (start..1).collect { level -> new Tick("ANVIL-1", level) }.iterator()
    }
}
