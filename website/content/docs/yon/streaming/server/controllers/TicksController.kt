import models.Tick

@RequestRoute("/ticks")
@Public
class TicksController {
    @StreamRoute
    fun streamTicks(): Sequence<Tick> = ticksUpTo(3)

    private fun ticksUpTo(last: Int): Sequence<Tick> = sequence {
        for (level in 1..last) {
            yield(Tick("ANVIL-1", level))
        }
    }
}
