using Models;

[RequestRoute("/ticks")]
[Public]
class TicksController {
    [StreamRoute]
    public IAsyncEnumerable<Tick> StreamTicks() {
        return TicksFor(new[] { "ANVIL-1", "ANVIL-2" });
    }

    private static async IAsyncEnumerable<Tick> TicksFor(string[] skus) {
        foreach (var sku in skus) {
            await Task.Delay(1000);
            yield return new Tick(sku, 3);
        }
    }
}
