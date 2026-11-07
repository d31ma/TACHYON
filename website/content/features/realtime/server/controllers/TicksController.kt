import models.Tick

@RequestRoute("/ticks")
@Public
class TicksController {
    @StreamRoute
    fun streamTicks(): Sequence<Tick> = countdown(3)

    private fun countdown(start: Int): Sequence<Tick> = sequence {
        for (level in start downTo 1) {
            yield(Tick("ANVIL-1", level))
        }
    }
}
