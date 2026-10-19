@Service
final class OrderService {
    @Needs
    private let ordersClient: OrdersClient

    func replace() async throws -> SentRequest {
        return try await ordersClient.replace(id: "a-7", order: .object(["item": .string("hammer")]), tenant: "north")
    }

    func change() async throws -> SentRequest {
        return try await ordersClient.change(id: "a-7", change: .object(["item": .string("nails")]), tenant: "south")
    }

    func remove() async throws -> SentRequest {
        return try await ordersClient.remove(id: "a-7", reason: "duplicate")
    }
}
