@HTTPClient("/ticks")
protocol TicksClient {
    @StreamRequest
    func receive(_ tick: Tick)
}
