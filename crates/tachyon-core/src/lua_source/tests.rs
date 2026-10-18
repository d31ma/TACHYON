use super::{LuaBody, LuaShadow, LuaStyle, LuaTypes};
use std::path::Path;

struct Sources;

impl Sources {
    const CONTROLLER: &'static str = "-- @RequestRoute(\"/orders\")\n-- @Public\n---@class OrdersController\nOrdersController = {\n  -- @Needs\n  ---@type OrderService\n  order_service = nil,\n}\n\n---@param id integer\n---@return Order[]\n-- @GetRoute(\"/{id}\")\nfunction OrdersController:get_orders(\n  -- @FromPath\n  id\n)\n  return self.order_service:list(id)\nend\n";
    const SHADOW: &'static str = "#[RequestRoute(\"/orders\")]\n#[Public]\nclass OrdersController\n  # @return [OrderService]\n  #[Needs]\n  attr_reader :order_service\n\n\n  # @param id [Integer]\n  # @return [Array<Order>]\n  #[GetRoute(\"/{id}\")]\n  def get_orders(\n    #[FromPath]\n    id\n  )\n    return self.order_service.list(id)\n  end\nend\n";
    const MODEL: &'static str =
        "---@class Order\n---@field name string\n---@field count integer\nOrder = {}\n";
    const MODEL_SHADOW: &'static str = "class Order\n  # @return [String]\n  attr_reader :name\n  # @return [Integer]\n  attr_reader :count\nend\n";
}

#[test]
fn a_lua_class_file_reads_as_a_ruby_class() {
    let shadow = LuaShadow::of(Sources::CONTROLLER);
    assert_eq!(shadow.text(), Sources::SHADOW);
    assert_eq!(shadow.first_problem(), None);
    assert_eq!(shadow.source_line(2), Some(3));
    assert_eq!(shadow.source_line(8), Some(9));
}

#[test]
fn luacats_fields_become_typed_ruby_attributes() {
    assert_eq!(LuaShadow::of(Sources::MODEL).text(), Sources::MODEL_SHADOW);
}

#[test]
fn luacats_types_read_as_yard_types() {
    for (lua, yard) in [
        ("string", "String"),
        ("integer", "Integer"),
        ("number", "Float"),
        ("boolean", "Boolean"),
        ("Order[]", "Array<Order>"),
        ("string?", "String, nil"),
        ("table<string, integer>", "Hash{String => Integer}"),
        ("string|nil", "String, nil"),
        ("", "void"),
    ] {
        assert_eq!(LuaTypes::yard(lua), yard, "{lua}");
    }
}

#[test]
fn lua_bodies_keep_their_words_and_hide_their_text() {
    let clean = LuaBody::sanitized("x = #items ~= 0 -- note\ny = \"a#b\" .. obj:call()\n");
    assert_eq!(
        clean,
        "x =  items != 0 #      \ny = \"a b\" .. obj.call()\n"
    );
    assert_eq!(
        LuaBody::translated("    elseif ok then local total = function(a) return a end"),
        "    elsif ok then total = proc do(a) return a end"
    );
    assert_eq!(LuaBody::translated("  until done"), "  end until done");
}

#[test]
fn forms_tachyon_cannot_read_are_refused_with_the_right_advice() {
    let path = Path::new("server/services/OrderService.lua");
    for (source, advice) in [
        ("local OrderService = {}\n", LuaShadow::LOCAL_TABLE),
        (
            "OrderService = {}\nreturn OrderService\n",
            LuaShadow::RETURNED_TABLE,
        ),
        ("OrderService = {}\nOther = {}\n", LuaShadow::SECOND_TYPE),
        (
            "OrderService = {}\nfunction Other:list()\nend\n",
            LuaShadow::FOREIGN_METHOD,
        ),
        (
            "OrderService = {}\nfunction OrderService:list() return 1 end\n",
            LuaShadow::ONE_LINE,
        ),
    ] {
        let failure = LuaStyle::validate(path, source)
            .err()
            .map(|failure| failure.to_string());
        let text = failure.unwrap_or_default();
        assert!(
            text.contains("TY1357") && text.contains(advice),
            "{source}: {text}"
        );
    }
    assert!(LuaStyle::validate(path, "OrderService = {}\n").is_ok());
}

#[test]
fn a_lua_test_unit_lowers_to_a_post_driver() {
    let program = "-- @Test(CatalogTestSetup, {\"partition\":\"tests\"})\n---@class CatalogServiceTest\nCatalogServiceTest = {\n  -- @Fake\n  ---@type CatalogService\n  catalog_service = nil,\n}\n\nfunction CatalogServiceTest:test_counts()\n  Assert.equals(2, 2)\nend\n\n\n-- @TestSetup\nCatalogTestSetup = {}\n\n---@param execution table\n---@return table\nfunction CatalogTestSetup.configure(execution)\n  return {}\nend\n\n\n-- @Service\nCatalogService = {}\n\n---@return integer\nfunction CatalogService:count()\n  return 2\nend\n";
    let lowered = super::LuaYon::lower(
        Path::new("server/tests/CatalogServiceTest.lua"),
        program,
        None,
    )
    .unwrap_or_else(|failure| panic!("{failure}"))
    .unwrap_or_else(|| panic!("a test unit"));
    assert!(
        lowered.source.contains("function Handler.POST(request)"),
        "{}",
        lowered.source
    );
    assert!(
        lowered.source.contains("unit.catalog_service = nil"),
        "{}",
        lowered.source
    );
}

#[test]
fn a_lua_controller_lowers_to_route_functions() {
    let lowered = super::LuaYon::lower(Path::new("server/controllers/OrdersController.lua"), "-- @RequestRoute(\"/orders\")\n-- @Public\nOrdersController = {}\n\n---@return integer\n-- @GetRoute\nfunction OrdersController:get_count()\n  return 2\nend\n", None)
        .unwrap_or_else(|failure| panic!("{failure}"))
        .unwrap_or_else(|| panic!("a controller"));
    assert!(
        lowered.source.contains("function Handler.GET(request)"),
        "{}",
        lowered.source
    );
    assert_eq!(lowered.routes.len(), 1);
}

#[test]
fn a_joined_program_holds_every_class() {
    let joined = LuaShadow::joined("A = {}\n\nB = {}\n");
    assert_eq!(joined.text(), "class A\nend\n\nclass B\nend\n");
    assert_eq!(joined.first_problem(), None);
}

#[test]
fn the_lua_behavior_runtime_passes_its_boundary_contract() {
    let Some(printed) = super::LuaProgram::printed(&[
        include_str!("lua_json.lua"),
        include_str!("lua_behaviors.lua"),
        include_str!("../../tests/fixtures/behavior/lua_runtime_contract.lua"),
    ]) else {
        return;
    };
    assert!(
        printed.contains("Lua behavior boundary contracts passed"),
        "{printed}"
    );
}

#[test]
fn a_lua_class_that_needs_a_resource_names_it_and_adr_0085() {
    for (service, name) in [
        (
            "-- @Service\nStockService = {\n  -- @Needs\n  ---@type DataSource\n  data_source = nil,\n}\n\n---@return integer\nfunction StockService:count()\n  return 1\nend\n",
            "DataSource",
        ),
        (
            "-- @Service\nStockService = {}\n\n---@return integer\nfunction StockService:count()\n  return relay(\"stock\", {})\nend\n",
            "relay",
        ),
    ] {
        let program = format!(
            "-- @RequestRoute(\"/stock\")\n-- @Public\nStockController = {{\n  -- @Needs\n  ---@type StockService\n  stock_service = nil,\n}}\n\n---@return integer\n-- @GetRoute\nfunction StockController:get_stock()\n  return self.stock_service:count()\nend\n\n\n{service}"
        );
        let failure = super::LuaYon::lower(
            Path::new("server/controllers/StockController.lua"),
            &program,
            None,
        )
        .err()
        .unwrap_or_else(|| panic!("{name} must be refused"));
        let text = failure.to_string();
        assert!(
            text.contains(&format!("Lua classes do not support {name} yet (ADR 0085)")),
            "{text}"
        );
    }
}

#[test]
fn a_lua_needs_all_prop_is_a_field_without_a_starting_value() {
    let fields = super::LuaFields::without_values(
        "-- @Service\n-- @NeedsAll\nDrawerService = {\n  ---@type boolean\n  open = nil,\n  ---@type integer\n  count = 0,\n  -- @Needs\n  ---@type CatalogService\n  catalog_service = nil,\n}\n",
        "DrawerService",
    );
    assert_eq!(
        fields,
        [
            (String::from("open"), String::from("boolean"), false),
            (
                String::from("catalog_service"),
                String::from("CatalogService"),
                true
            ),
        ]
    );
}

#[test]
fn a_lua_table_closed_on_its_own_line_has_no_fields() {
    for opened in ["GreetingClient = {}", "GreetingClient = { }"] {
        let source = format!(
            "---@class GreetingClient\n{opened}\n\nfunction GreetingClient:read()\n  local query = {{\n    name = nil,\n  }}\n  return query\nend\n"
        );
        let fields = super::LuaFields::without_values(&source, "GreetingClient");
        assert_eq!(fields, [], "{opened}");
    }
}
