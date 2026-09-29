use super::{LuaBody, LuaShadow, LuaStyle, LuaTypes};
use std::path::Path;

struct Sources;

impl Sources {
    const CONTROLLER: &'static str = "-- @RequestRoute(\"/orders\")\n-- @Public\n---@class OrdersController\nOrdersController = {\n  -- @Needs\n  ---@type OrderService\n  order_service = nil,\n}\n\n---@param id integer\n---@return Order[]\n-- @GetRoute(\"/{id}\")\nfunction OrdersController:get_orders(\n  -- @FromPath\n  id\n)\n  return self.order_service:list(id)\nend\n";
    const SHADOW: &'static str = "#[RequestRoute(\"/orders\")]\n#[Public]\n\nclass OrdersController\n  #[Needs]\n  # @return [OrderService]\n  attr_reader :order_service\n\n\n  # @param id [Integer]\n  # @return [Array<Order>]\n  #[GetRoute(\"/{id}\")]\n  def get_orders(\n    #[FromPath]\n    id\n  )\n    return self.order_service.list(id)\n  end\nend\n";
    const MODEL: &'static str = "---@class Order\n---@field name string\n---@field count integer\nOrder = {}\n";
    const MODEL_SHADOW: &'static str = "\n\n\nclass Order\n  # @return [String]\n  attr_reader :name\n  # @return [Integer]\n  attr_reader :count\nend\n";
}

#[test]
fn a_lua_class_file_reads_as_a_ruby_class() {
    let shadow = LuaShadow::of(Sources::CONTROLLER);
    assert_eq!(shadow.text(), Sources::SHADOW);
    assert_eq!(shadow.first_problem(), None);
    assert_eq!(shadow.source_line(3), Some(3));
    assert_eq!(shadow.source_line(9), Some(9));
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
    assert_eq!(clean, "x =  items != 0 #      \ny = \"a b\" .. obj.call()\n");
    assert_eq!(
        LuaBody::translated("    elseif ok then local total = function(a) return a end"),
        "    elsif ok then total = proc do |a| return a end"
    );
    assert_eq!(LuaBody::translated("  until done"), "  end until done");
}

#[test]
fn forms_tachyon_cannot_read_are_refused_with_the_right_advice() {
    let path = Path::new("server/services/OrderService.lua");
    for (source, advice) in [
        ("local OrderService = {}\n", LuaShadow::LOCAL_TABLE),
        ("OrderService = {}\nreturn OrderService\n", LuaShadow::RETURNED_TABLE),
        ("OrderService = {}\nOther = {}\n", LuaShadow::SECOND_TYPE),
        ("OrderService = {}\nfunction Other:list()\nend\n", LuaShadow::FOREIGN_METHOD),
        ("OrderService = {}\nfunction OrderService:list() return 1 end\n", LuaShadow::ONE_LINE),
    ] {
        let failure = LuaStyle::validate(path, source).err().map(|failure| failure.to_string());
        let text = failure.unwrap_or_default();
        assert!(text.contains("TY1357") && text.contains(advice), "{source}: {text}");
    }
    assert!(LuaStyle::validate(path, "OrderService = {}\n").is_ok());
}

#[test]
fn a_joined_program_holds_every_class() {
    let joined = LuaShadow::joined("A = {}\n\nB = {}\n");
    assert_eq!(joined.text(), "class A\n\nend\nclass B\nend\n");
    assert_eq!(joined.first_problem(), None);
}
