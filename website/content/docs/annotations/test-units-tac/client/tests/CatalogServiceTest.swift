@Test(CatalogTestSetup, {"partition":"tests"})
@UseProfiles("test")
final class CatalogServiceTest {
    func testHasAContext() async throws {
        try Assert.isTrue(behaviors != nil, "a context")
    }
    func twice(_ value: Int) -> Int { value * 2 }
    func testDoublesAValue() async throws {
        try Assert.equals(8, twice(4))
    }
}
