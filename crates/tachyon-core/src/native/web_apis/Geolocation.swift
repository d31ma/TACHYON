final class Geolocation {
    func currentPosition() throws {
        _ = try TacWebHost.call("geolocation", "currentPosition")
    }
}
