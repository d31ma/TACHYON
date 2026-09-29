@Service
final class PlaceService {
    private let geolocation = Geolocation()
    private let clipboard = Clipboard()
    var place: String = ""
    var problem: String = ""

    func locate() {
        keepProblem { try geolocation.currentPosition() }
    }

    func copy() {
        keepProblem { try clipboard.writeText(place) }
    }

    @OnEvent
    func found(_ found: PositionFound) {
        place = "\(found.latitude), \(found.longitude)"
    }

    @OnEvent
    func failed(_ failed: BrowserCallFailed) {
        problem = failed.message
    }

    private func keepProblem(_ call: () throws -> Void) {
        do {
            try call()
        } catch {
            problem = String(describing: error)
        }
    }
}
