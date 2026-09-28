@Test(CatalogTestSetup, {"partition":"tests"})
@UseProfiles("test")
final class CatalogServiceTest {
    func testHasAContext() async throws {
        if behaviors == nil { throw BehaviorError("NO_CONTEXT") }
    }
    func twice(_ value: Int) -> Int { value * 2 }
    func testDoublesAValue() async throws {
        if twice(4) != 8 { throw BehaviorError("EXPECTED_8") }
    }
}
