@MainActor
final class AppleCurrentPosition: NSObject, CLLocationManagerDelegate {
    static let denied = "User denied Geolocation"
    private let manager = CLLocationManager()
    private var replies: [(String) -> Void] = []
    private var deadline: Timer?
    private var asking = false

    func find(_ timeout: TimeInterval, _ reply: @escaping (String) -> Void) {
        replies.append(reply)
        guard replies.count == 1 else { return }
        manager.delegate = self
        manager.desiredAccuracy = kCLLocationAccuracyBest
        deadline = Timer.scheduledTimer(withTimeInterval: timeout, repeats: false) { [weak self] _ in
            MainActor.assumeIsolated { self?.finish(AppleBrowserApis.failure("Timeout expired")) }
        }
        decide()
    }

    nonisolated func locationManagerDidChangeAuthorization(_ manager: CLLocationManager) {
        MainActor.assumeIsolated {
            guard asking, manager.authorizationStatus != .notDetermined else { return }
            asking = false
            decide()
        }
    }

    nonisolated func locationManager(_ manager: CLLocationManager, didUpdateLocations locations: [CLLocation]) {
        MainActor.assumeIsolated {
            guard let location = locations.last else { return }
            let coords: [String: Double] = [
                "latitude": location.coordinate.latitude,
                "longitude": location.coordinate.longitude,
                "accuracy": location.horizontalAccuracy,
            ]
            let timestamp = (location.timestamp.timeIntervalSince1970 * 1000).rounded()
            finish(AppleBrowserApis.success(["coords": coords, "timestamp": timestamp] as [String: Any]))
        }
    }

    nonisolated func locationManager(_ manager: CLLocationManager, didFailWithError error: Error) {
        MainActor.assumeIsolated {
            let refused = (error as? CLError)?.code == .denied
            finish(AppleBrowserApis.failure(refused ? Self.denied : "Position unavailable: " + error.localizedDescription))
        }
    }

    private func decide() {
        switch manager.authorizationStatus {
        case .notDetermined:
            asking = true
            manager.requestWhenInUseAuthorization()
        case .denied, .restricted:
            finish(AppleBrowserApis.failure(Self.denied))
        default:
            manager.requestLocation()
        }
    }

    private func finish(_ answer: String) {
        deadline?.invalidate()
        deadline = nil
        asking = false
        let waiting = replies
        replies = []
        waiting.forEach { $0(answer) }
    }
}
