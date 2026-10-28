#![allow(clippy::expect_used)]

use super::tests::Project;

struct Typed;

impl Typed {
    const JAVA_SERVICE: &str = "@Service\nclass TaxService {\n    int rate(String region) {\n        return 2;\n    }\n\n    int rate(String region, int year) {\n        return year;\n    }\n\n    public void record(@CacheKey String sku, int count) {\n        System.out.println(sku);\n    }\n\n    private int secret() {\n        return 1;\n    }\n}\n";
    const JAVA_DELEGATE: &str = "@Delegate\nclass TaxDelegate {\n    @Needs\n    private final TaxService taxes;\n\n    int total(String region) {\n        return taxes.rate(region) * 2;\n    }\n}\n";
    const GROOVY_SERVICE: &str = "@Service\nclass TaxService {\n    int rate(String region) {\n        return 2\n    }\n}\n";
    const GROOVY_DELEGATE: &str = "@Delegate\nclass TaxDelegate {\n    @Needs\n    private TaxService taxes\n\n    int total(String region) {\n        return taxes.rate(region) * 2\n    }\n}\n";
    const KOTLIN_SERVICE: &str = "@Service\nclass TaxService {\n    fun rate(region: String): Int {\n        return 2\n    }\n\n    fun rates(vararg regions: String): Int = regions.size\n\n    private fun secret(): Int {\n        return 1\n    }\n}\n";
    const KOTLIN_DELEGATE: &str = "@Delegate\nclass TaxDelegate {\n    @Needs\n    private val taxes: TaxService\n\n    fun rate(region: String): Int {\n        return taxes.rate(region) * 2\n    }\n}\n";
    const GO_SERVICE: &str = "// @Service\ntype TaxService struct {\n\tCount int\n}\n\nfunc (service *TaxService) Rate(region string) int {\n\treturn len(region)\n}\n\nfunc (service *TaxService) Record(names ...string) {\n\t_ = names\n}\n\nfunc (service *TaxService) secret() int {\n\treturn 1\n}\n";
    const GO_DELEGATE: &str = "// @Delegate\ntype TaxDelegate struct {\n\t// @Needs\n\ttaxes *TaxService\n}\n\nfunc (delegate *TaxDelegate) Total(region string) int {\n\treturn delegate.taxes.Rate(region) * 2\n}\n";
    const RUST_SERVICE: &str = "#[Service]\nstruct TaxService {\n    count: usize,\n}\n\nimpl TaxService {\n    pub fn rate(&self, region: &str) -> i64 {\n        region.len() as i64\n    }\n\n    pub async fn fetch(&self, id: u32) -> String {\n        id.to_string()\n    }\n\n    fn secret(&self) -> i64 {\n        1\n    }\n}\n";
    const RUST_DELEGATE: &str = "#[Delegate]\nstruct TaxDelegate {\n    #[Needs]\n    taxes: TaxService,\n}\n\nimpl TaxDelegate {\n    pub fn total(&self, region: &str) -> i64 {\n        self.taxes.rate(region) * 2\n    }\n}\n";
    const CSHARP_SERVICE: &str = "[Service]\npublic sealed class TaxService {\n    public int Rate(string region) {\n        return 2;\n    }\n\n    public async Task<int> Fetch([CacheKey] string id) {\n        return await Task.FromResult(id.Length);\n    }\n\n    public void Log(string text) => System.Console.WriteLine(text);\n\n    private int Secret() {\n        return 1;\n    }\n}\n";
    const CSHARP_DELEGATE: &str = "[Delegate]\npublic sealed class TaxDelegate {\n    [Needs]\n    private readonly TaxService _taxes;\n\n    public int Total(string region) {\n        return _taxes.Rate(region) * 2;\n    }\n}\n";
    const DART_SERVICE: &str = "@Service\nclass TaxService {\n  int rate(String region, {int year = 0, int month = 1}) {\n    return year + month;\n  }\n\n  Future<int> fetch(int id) async {\n    return id;\n  }\n\n  int _secret() {\n    return 1;\n  }\n}\n";
    const DART_DELEGATE: &str = "@Delegate\nclass TaxDelegate {\n  @Needs\n  final TaxService taxes;\n\n  int total(String region) {\n    return taxes.rate(region) * 2;\n  }\n}\n";
    const SWIFT_SERVICE: &str = "@Service\nfinal class TaxService {\n    func rate(_ region: String, year: Int) -> Int {\n        return year\n    }\n\n    func fetch(id: Int) async throws -> String {\n        return String(id)\n    }\n\n    private func secret() -> Int {\n        return 1\n    }\n}\n";
    const SWIFT_DELEGATE: &str = "@Delegate\nfinal class TaxDelegate {\n    @Needs\n    private let taxes: TaxService\n\n    func total(_ region: String) -> Int {\n        return taxes.rate(region, year: 1) * 2\n    }\n}\n";
    const CPP_SERVICE: &str = "#pragma once\n\nclass [[Service]] TaxService {\npublic:\n    int rate(int year) const {\n        return year;\n    }\n\n    int rate(int year, int month) const {\n        return year + month;\n    }\n\n    void record(const std::string& name) {\n        last = name;\n    }\n\n    std::string last;\n};\n";
    const CPP_DELEGATE: &str = "#pragma once\n#include \"../services/TaxService.hpp\"\n\nclass [[Delegate]] TaxDelegate {\npublic:\n    [[Needs]] TaxService& taxes;\n\n    int total(int year) const {\n        return taxes.rate(year) * 2;\n    }\n};\n";
    const FSHARP_SERVICE: &str = "[<Service>]\ntype TaxService() =\n    member this.Rate(region: string) : int =\n        region.Length\n\n    [<OnEvent>]\n    member this.Listen(region: string) : unit =\n        ()\n";
    const FSHARP_DELEGATE: &str = "[<Delegate>]\ntype TaxDelegate() =\n    [<Needs>]\n    member val TaxService : TaxService = Unchecked.defaultof<TaxService> with get, set\n\n    member this.Total(region: string) : int =\n        this.TaxService.Rate(region) * 2\n";
    const LUA_SERVICE: &str = "-- @Service\n---@class TaxService\nTaxService = {}\n\n---@param region string\n---@return integer\nfunction TaxService:rate(region)\n  return 2\nend\n\nfunction TaxService:_secret()\n  return 1\nend\n";
    const LUA_DELEGATE: &str = "-- @Delegate\n---@class TaxDelegate\nTaxDelegate = {\n  -- @Needs\n  ---@type TaxService\n  tax_service = nil,\n}\n\nfunction TaxDelegate:total(region)\n  return self.tax_service:rate(region) * 2\nend\n";
    const JAVA_STAGED: &str = "@Delegate\nclass TaxDelegate {\n    @Needs\n    private final TaxService taxes;\n\n    int total(String region) {\n        return taxes.rate(region) * 2;\n    }\n\n  int rate(String region) {\n    return taxes.rate(region);\n  }\n\n  int rate(String region, int year) {\n    return taxes.rate(region, year);\n  }\n\n  public void record(String sku, int count) {\n    taxes.record(sku, count);\n  }\n}\n";
    const GROOVY_STAGED: &str = "@Delegate\nclass TaxDelegate {\n    @Needs\n    private TaxService taxes\n\n    int total(String region) {\n        return taxes.rate(region) * 2\n    }\n\n  int rate(String region) {\n    return taxes.rate(region);\n  }\n}\n";
    const KOTLIN_STAGED: &str = "@Delegate\nclass TaxDelegate {\n    @Needs\n    private val taxes: TaxService\n\n    fun rate(region: String): Int {\n        return taxes.rate(region) * 2\n    }\n\n  fun rates(vararg regions: String): Int = taxes.rates(*regions)\n}\n";
    const GO_STAGED: &str = "// @Delegate\ntype TaxDelegate struct {\n\t// @Needs\n\ttaxes *TaxService\n}\n\nfunc (delegate *TaxDelegate) Total(region string) int {\n\treturn delegate.taxes.Rate(region) * 2\n}\n\nfunc (delegate *TaxDelegate) Rate(region string) int {\n\treturn delegate.taxes.Rate(region)\n}\n\nfunc (delegate *TaxDelegate) Record(names ...string) {\n\tdelegate.taxes.Record(names...)\n}\n";
    const RUST_STAGED: &str = "#[Delegate]\nstruct TaxDelegate {\n    #[Needs]\n    taxes: TaxService,\n}\n\nimpl TaxDelegate {\n    pub fn total(&self, region: &str) -> i64 {\n        self.taxes.rate(region) * 2\n    }\n}\n\nimpl TaxDelegate {\n    pub fn rate(&self, region: &str) -> i64 {\n        self.taxes.rate(region)\n    }\n\n    pub async fn fetch(&self, id: u32) -> String {\n        self.taxes.fetch(id).await\n    }\n}\n";
    const CSHARP_STAGED: &str = "[Delegate]\npublic sealed class TaxDelegate {\n    [Needs]\n    private readonly TaxService _taxes;\n\n    public int Total(string region) {\n        return _taxes.Rate(region) * 2;\n    }\n\n  public int Rate(string region) {\n    return _taxes.Rate(region);\n  }\n\n  public Task<int> Fetch(string id) {\n    return _taxes.Fetch(id);\n  }\n\n  public void Log(string text) {\n    _taxes.Log(text);\n  }\n}\n";
    const DART_STAGED: &str = "@Delegate\nclass TaxDelegate {\n  @Needs\n  final TaxService taxes;\n\n  int total(String region) {\n    return taxes.rate(region) * 2;\n  }\n\n  int rate(String region, {int year = 0, int month = 1}) {\n    return taxes.rate(region, year: year, month: month);\n  }\n\n  Future<int> fetch(int id) {\n    return taxes.fetch(id);\n  }\n}\n";
    const SWIFT_STAGED: &str = "@Delegate\nfinal class TaxDelegate {\n    @Needs\n    private let taxes: TaxService\n\n    func total(_ region: String) -> Int {\n        return taxes.rate(region, year: 1) * 2\n    }\n\n  func rate(_ region: String, year: Int) -> Int {\n    return taxes.rate(region, year: year)\n  }\n\n  func fetch(id: Int) async throws -> String {\n    return try await taxes.fetch(id: id)\n  }\n}\n";
    const CPP_STAGED: &str = "#pragma once\n#include \"../services/TaxService.hpp\"\n\nclass [[Delegate]] TaxDelegate {\npublic:\n    [[Needs]] TaxService& taxes;\n\n    int total(int year) const {\n        return taxes.rate(year) * 2;\n    }\n\npublic:\n  int rate(int year) const {\n    return taxes.rate(year);\n  }\n\n  int rate(int year, int month) const {\n    return taxes.rate(year, month);\n  }\n\n  void record(const std::string& name) {\n    taxes.record(name);\n  }\n};\n";
    const FSHARP_STAGED: &str = "[<Delegate>]\ntype TaxDelegate() =\n    [<Needs>]\n    member val TaxService : TaxService = Unchecked.defaultof<TaxService> with get, set\n\n    member this.Total(region: string) : int =\n        this.TaxService.Rate(region) * 2\n    member this.Rate(region: string) : int = this.TaxService.Rate(region)\n\n";
    const LUA_STAGED: &str = "-- @Delegate\n---@class TaxDelegate\nTaxDelegate = {\n  -- @Needs\n  ---@type TaxService\n  tax_service = nil,\n}\n\nfunction TaxDelegate:total(region)\n  return self.tax_service:rate(region) * 2\nend\n\nfunction TaxDelegate:rate(...)\n  return self.tax_service:rate(...)\nend\n";

    fn staged(language: &str, (service, delegate): (&str, &str)) -> String {
        let service_path = format!("server/services/TaxService.{language}");
        let delegate_path = format!("server/delegates/TaxDelegate.{language}");
        let files = [(service_path.as_str(), service), (delegate_path.as_str(), delegate)];
        Project::staged(&files, &delegate_path).expect("forwards")
    }

    fn client_staged(language: &str, (service, delegate): (&str, &str)) -> String {
        let service_path = format!("client/services/TaxService.{language}");
        let delegate_path = format!("client/delegates/TaxDelegate.{language}");
        let files = [(service_path.as_str(), service), (delegate_path.as_str(), delegate)];
        Project::staged(&files, &delegate_path).expect("forwards")
    }
}


#[test]
fn a_java_delegate_takes_on_each_public_overload_with_its_own_signature() {
    let staged = Typed::staged("java", (Typed::JAVA_SERVICE, Typed::JAVA_DELEGATE));
    assert_eq!(staged, Typed::JAVA_STAGED);
}

#[test]
fn a_groovy_delegate_takes_on_the_methods_of_its_service() {
    let staged = Typed::staged("groovy", (Typed::GROOVY_SERVICE, Typed::GROOVY_DELEGATE));
    assert_eq!(staged, Typed::GROOVY_STAGED);
}

#[test]
fn a_kotlin_delegate_keeps_its_own_method_and_spreads_a_vararg() {
    let staged = Typed::staged("kt", (Typed::KOTLIN_SERVICE, Typed::KOTLIN_DELEGATE));
    assert_eq!(staged, Typed::KOTLIN_STAGED);
}

#[test]
fn a_go_delegate_gets_methods_on_its_own_receiver() {
    let staged = Typed::staged("go", (Typed::GO_SERVICE, Typed::GO_DELEGATE));
    assert_eq!(staged, Typed::GO_STAGED);
}

#[test]
fn a_rust_delegate_gets_an_impl_block_that_awaits_async_methods() {
    let staged = Typed::staged("rs", (Typed::RUST_SERVICE, Typed::RUST_DELEGATE));
    assert_eq!(staged, Typed::RUST_STAGED);
}

#[test]
fn a_csharp_delegate_drops_async_and_markers_from_the_copied_signature() {
    let staged = Typed::staged("cs", (Typed::CSHARP_SERVICE, Typed::CSHARP_DELEGATE));
    assert_eq!(staged, Typed::CSHARP_STAGED);
}

#[test]
fn a_dart_delegate_passes_named_arguments_by_name() {
    let staged = Typed::client_staged("dart", (Typed::DART_SERVICE, Typed::DART_DELEGATE));
    assert_eq!(staged, Typed::DART_STAGED);
}

#[test]
fn a_swift_delegate_keeps_labels_and_tries_and_awaits() {
    let staged = Typed::client_staged("swift", (Typed::SWIFT_SERVICE, Typed::SWIFT_DELEGATE));
    assert_eq!(staged, Typed::SWIFT_STAGED);
}

#[test]
fn a_cpp_delegate_adds_a_public_section_with_each_overload() {
    let staged = Typed::client_staged("hpp", (Typed::CPP_SERVICE, Typed::CPP_DELEGATE));
    assert_eq!(staged, Typed::CPP_STAGED);
}

#[test]
fn an_fsharp_delegate_calls_its_service_property_through_this() {
    let staged = Typed::client_staged("fs", (Typed::FSHARP_SERVICE, Typed::FSHARP_DELEGATE));
    assert_eq!(staged, Typed::FSHARP_STAGED);
}

#[test]
fn a_lua_delegate_gets_functions_that_pass_every_argument_on() {
    let staged = Typed::client_staged("lua", (Typed::LUA_SERVICE, Typed::LUA_DELEGATE));
    assert_eq!(staged, Typed::LUA_STAGED);
}

