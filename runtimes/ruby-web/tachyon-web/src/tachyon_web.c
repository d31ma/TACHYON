#include <mruby.h>
#include <mruby/array.h>
#include <mruby/class.h>
#include <mruby/error.h>
#include <mruby/hash.h>
#include <mruby/irep.h>
#include <mruby/numeric.h>
#include <mruby/string.h>
#include <mruby/variable.h>
#include <mruby/compile.h>
#include <emscripten.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

__attribute__((import_module("tachyon"), import_name("tac_host_emit")))
void tac_host_emit(const char *pointer, size_t length);

typedef struct {
  const char *start;
  const char *cursor;
  const char *end;
  mrb_bool symbolize;
  int depth;
} json_reader;

#define JSON_MAX_NESTING 100

static struct RClass *
json_error_class(mrb_state *mrb, const char *name)
{
  struct RClass *json = mrb_module_get(mrb, "JSON");
  return mrb_class_get_under(mrb, json, name);
}

static void
json_fail(mrb_state *mrb, json_reader *reader, const char *what)
{
  mrb_raisef(mrb, json_error_class(mrb, "ParserError"), "%s at offset %i",
             what, (mrb_int)(reader->cursor - reader->start));
}

static void
json_skip_space(json_reader *reader)
{
  while (reader->cursor < reader->end) {
    char c = *reader->cursor;
    if (c != ' ' && c != '\t' && c != '\n' && c != '\r') return;
    reader->cursor++;
  }
}

static mrb_value json_read_value(mrb_state *mrb, json_reader *reader);

static int
json_hex(mrb_state *mrb, json_reader *reader)
{
  int value = 0;
  if (reader->end - reader->cursor < 4) json_fail(mrb, reader, "incomplete unicode escape");
  for (int i = 0; i < 4; i++) {
    char c = *reader->cursor++;
    value <<= 4;
    if (c >= '0' && c <= '9') value |= c - '0';
    else if (c >= 'a' && c <= 'f') value |= c - 'a' + 10;
    else if (c >= 'A' && c <= 'F') value |= c - 'A' + 10;
    else json_fail(mrb, reader, "invalid unicode escape");
  }
  return value;
}

static void
json_append_code_point(mrb_state *mrb, mrb_value out, uint32_t code)
{
  char bytes[4];
  int size;
  if (code < 0x80) { bytes[0] = (char)code; size = 1; }
  else if (code < 0x800) { bytes[0] = (char)(0xC0 | (code >> 6)); bytes[1] = (char)(0x80 | (code & 0x3F)); size = 2; }
  else if (code < 0x10000) { bytes[0] = (char)(0xE0 | (code >> 12)); bytes[1] = (char)(0x80 | ((code >> 6) & 0x3F)); bytes[2] = (char)(0x80 | (code & 0x3F)); size = 3; }
  else { bytes[0] = (char)(0xF0 | (code >> 18)); bytes[1] = (char)(0x80 | ((code >> 12) & 0x3F)); bytes[2] = (char)(0x80 | ((code >> 6) & 0x3F)); bytes[3] = (char)(0x80 | (code & 0x3F)); size = 4; }
  mrb_str_cat(mrb, out, bytes, size);
}

static mrb_value
json_read_string(mrb_state *mrb, json_reader *reader)
{
  mrb_value out = mrb_str_new(mrb, NULL, 0);
  reader->cursor++;
  for (;;) {
    const char *run = reader->cursor;
    while (reader->cursor < reader->end && *reader->cursor != '"' && *reader->cursor != '\\' && (unsigned char)*reader->cursor >= 0x20) {
      reader->cursor++;
    }
    if (reader->cursor > run) mrb_str_cat(mrb, out, run, reader->cursor - run);
    if (reader->cursor >= reader->end) json_fail(mrb, reader, "unterminated string");
    char c = *reader->cursor++;
    if (c == '"') return out;
    if (c != '\\') { reader->cursor--; json_fail(mrb, reader, "control character in string"); }
    if (reader->cursor >= reader->end) json_fail(mrb, reader, "unterminated escape");
    char escape = *reader->cursor++;
    switch (escape) {
      case '"': mrb_str_cat_lit(mrb, out, "\""); break;
      case '\\': mrb_str_cat_lit(mrb, out, "\\"); break;
      case '/': mrb_str_cat_lit(mrb, out, "/"); break;
      case 'b': mrb_str_cat_lit(mrb, out, "\b"); break;
      case 'f': mrb_str_cat_lit(mrb, out, "\f"); break;
      case 'n': mrb_str_cat_lit(mrb, out, "\n"); break;
      case 'r': mrb_str_cat_lit(mrb, out, "\r"); break;
      case 't': mrb_str_cat_lit(mrb, out, "\t"); break;
      case 'u': {
        uint32_t code = (uint32_t)json_hex(mrb, reader);
        if (code >= 0xD800 && code < 0xDC00) {
          if (reader->end - reader->cursor < 6 || reader->cursor[0] != '\\' || reader->cursor[1] != 'u') {
            json_fail(mrb, reader, "incomplete surrogate pair");
          }
          reader->cursor += 2;
          uint32_t low = (uint32_t)json_hex(mrb, reader);
          if (low < 0xDC00 || low >= 0xE000) json_fail(mrb, reader, "invalid surrogate pair");
          code = 0x10000 + ((code - 0xD800) << 10) + (low - 0xDC00);
        }
        json_append_code_point(mrb, out, code);
        break;
      }
      default: json_fail(mrb, reader, "unknown escape");
    }
  }
}

static mrb_value
json_read_number(mrb_state *mrb, json_reader *reader)
{
  const char *begin = reader->cursor;
  mrb_bool fractional = FALSE;
  if (*reader->cursor == '-') reader->cursor++;
  if (reader->cursor >= reader->end || *reader->cursor < '0' || *reader->cursor > '9') json_fail(mrb, reader, "invalid number");
  if (*reader->cursor == '0') reader->cursor++;
  else while (reader->cursor < reader->end && *reader->cursor >= '0' && *reader->cursor <= '9') reader->cursor++;
  if (reader->cursor < reader->end && *reader->cursor == '.') {
    fractional = TRUE;
    reader->cursor++;
    if (reader->cursor >= reader->end || *reader->cursor < '0' || *reader->cursor > '9') json_fail(mrb, reader, "invalid number");
    while (reader->cursor < reader->end && *reader->cursor >= '0' && *reader->cursor <= '9') reader->cursor++;
  }
  if (reader->cursor < reader->end && (*reader->cursor == 'e' || *reader->cursor == 'E')) {
    fractional = TRUE;
    reader->cursor++;
    if (reader->cursor < reader->end && (*reader->cursor == '+' || *reader->cursor == '-')) reader->cursor++;
    if (reader->cursor >= reader->end || *reader->cursor < '0' || *reader->cursor > '9') json_fail(mrb, reader, "invalid number");
    while (reader->cursor < reader->end && *reader->cursor >= '0' && *reader->cursor <= '9') reader->cursor++;
  }
  char buffer[64];
  size_t length = (size_t)(reader->cursor - begin);
  if (!fractional && length < 19) {
    memcpy(buffer, begin, length);
    buffer[length] = '\0';
    return mrb_int_value(mrb, (mrb_int)strtoll(buffer, NULL, 10));
  }
  if (!fractional) {
    return mrb_str_to_integer(mrb, mrb_str_new(mrb, begin, length), 10, FALSE);
  }
  if (length >= sizeof(buffer)) json_fail(mrb, reader, "number too long");
  memcpy(buffer, begin, length);
  buffer[length] = '\0';
  return mrb_float_value(mrb, strtod(buffer, NULL));
}

static void
json_expect_word(mrb_state *mrb, json_reader *reader, const char *word)
{
  size_t length = strlen(word);
  if ((size_t)(reader->end - reader->cursor) < length || memcmp(reader->cursor, word, length) != 0) json_fail(mrb, reader, "unexpected token");
  reader->cursor += length;
}

static mrb_value
json_read_value(mrb_state *mrb, json_reader *reader)
{
  json_skip_space(reader);
  if (reader->cursor >= reader->end) json_fail(mrb, reader, "unexpected end of input");
  char c = *reader->cursor;
  if (c == '"') return json_read_string(mrb, reader);
  if (c == '-' || (c >= '0' && c <= '9')) return json_read_number(mrb, reader);
  if (c == 't') { json_expect_word(mrb, reader, "true"); return mrb_true_value(); }
  if (c == 'f') { json_expect_word(mrb, reader, "false"); return mrb_false_value(); }
  if (c == 'n') { json_expect_word(mrb, reader, "null"); return mrb_nil_value(); }
  if (c != '[' && c != '{') json_fail(mrb, reader, "unexpected token");
  if (++reader->depth > JSON_MAX_NESTING) mrb_raisef(mrb, json_error_class(mrb, "NestingError"), "nesting of %i is too deep", (mrb_int)reader->depth);
  reader->cursor++;
  int arena_index = mrb_gc_arena_save(mrb);
  mrb_value result;
  if (c == '[') {
    result = mrb_ary_new(mrb);
    json_skip_space(reader);
    if (reader->cursor < reader->end && *reader->cursor == ']') reader->cursor++;
    else for (;;) {
      mrb_ary_push(mrb, result, json_read_value(mrb, reader));
      mrb_gc_arena_restore(mrb, arena_index);
      mrb_gc_protect(mrb, result);
      json_skip_space(reader);
      if (reader->cursor < reader->end && *reader->cursor == ',') { reader->cursor++; continue; }
      if (reader->cursor < reader->end && *reader->cursor == ']') { reader->cursor++; break; }
      json_fail(mrb, reader, "expected ',' or ']'");
    }
  }
  else {
    result = mrb_hash_new(mrb);
    json_skip_space(reader);
    if (reader->cursor < reader->end && *reader->cursor == '}') reader->cursor++;
    else for (;;) {
      json_skip_space(reader);
      if (reader->cursor >= reader->end || *reader->cursor != '"') json_fail(mrb, reader, "expected object key");
      mrb_value key = json_read_string(mrb, reader);
      if (reader->symbolize) key = mrb_symbol_value(mrb_intern_str(mrb, key));
      json_skip_space(reader);
      if (reader->cursor >= reader->end || *reader->cursor != ':') json_fail(mrb, reader, "expected ':'");
      reader->cursor++;
      mrb_value value = json_read_value(mrb, reader);
      mrb_hash_set(mrb, result, key, value);
      mrb_gc_arena_restore(mrb, arena_index);
      mrb_gc_protect(mrb, result);
      json_skip_space(reader);
      if (reader->cursor < reader->end && *reader->cursor == ',') { reader->cursor++; continue; }
      if (reader->cursor < reader->end && *reader->cursor == '}') { reader->cursor++; break; }
      json_fail(mrb, reader, "expected ',' or '}'");
    }
  }
  reader->depth--;
  return result;
}

static mrb_value
json_parse(mrb_state *mrb, mrb_value self)
{
  mrb_value source;
  mrb_value options = mrb_nil_value();
  mrb_get_args(mrb, "S|H", &source, &options);
  json_reader reader;
  reader.start = RSTRING_PTR(source);
  reader.cursor = reader.start;
  reader.end = reader.start + RSTRING_LEN(source);
  reader.depth = 0;
  reader.symbolize = FALSE;
  if (mrb_hash_p(options)) {
    reader.symbolize = mrb_test(mrb_hash_get(mrb, options, mrb_symbol_value(mrb_intern_lit(mrb, "symbolize_names"))));
  }
  mrb_value result = json_read_value(mrb, &reader);
  json_skip_space(&reader);
  if (reader.cursor != reader.end) json_fail(mrb, &reader, "unexpected trailing content");
  return result;
}

static void json_write(mrb_state *mrb, mrb_value out, mrb_value value, mrb_value indent, int depth);

static void
json_write_string(mrb_state *mrb, mrb_value out, const char *pointer, mrb_int length)
{
  static const char hex[] = "0123456789abcdef";
  mrb_str_cat_lit(mrb, out, "\"");
  const char *run = pointer;
  const char *end = pointer + length;
  for (const char *cursor = pointer; cursor < end; cursor++) {
    unsigned char c = (unsigned char)*cursor;
    if (c >= 0x20 && c != '"' && c != '\\') continue;
    if (cursor > run) mrb_str_cat(mrb, out, run, cursor - run);
    run = cursor + 1;
    switch (c) {
      case '"': mrb_str_cat_lit(mrb, out, "\\\""); break;
      case '\\': mrb_str_cat_lit(mrb, out, "\\\\"); break;
      case '\b': mrb_str_cat_lit(mrb, out, "\\b"); break;
      case '\f': mrb_str_cat_lit(mrb, out, "\\f"); break;
      case '\n': mrb_str_cat_lit(mrb, out, "\\n"); break;
      case '\r': mrb_str_cat_lit(mrb, out, "\\r"); break;
      case '\t': mrb_str_cat_lit(mrb, out, "\\t"); break;
      default: {
        char escape[6] = { '\\', 'u', '0', '0', hex[c >> 4], hex[c & 15] };
        mrb_str_cat(mrb, out, escape, 6);
      }
    }
  }
  if (end > run) mrb_str_cat(mrb, out, run, end - run);
  mrb_str_cat_lit(mrb, out, "\"");
}

static void
json_newline(mrb_state *mrb, mrb_value out, mrb_value indent, int depth)
{
  if (mrb_nil_p(indent)) return;
  mrb_str_cat_lit(mrb, out, "\n");
  for (int i = 0; i < depth; i++) mrb_str_cat_str(mrb, out, indent);
}

static void
json_write(mrb_state *mrb, mrb_value out, mrb_value value, mrb_value indent, int depth)
{
  if (depth > JSON_MAX_NESTING) mrb_raisef(mrb, json_error_class(mrb, "NestingError"), "nesting of %i is too deep", (mrb_int)depth);
  int arena_index = mrb_gc_arena_save(mrb);
  if (mrb_nil_p(value)) mrb_str_cat_lit(mrb, out, "null");
  else if (mrb_true_p(value)) mrb_str_cat_lit(mrb, out, "true");
  else if (mrb_false_p(value)) mrb_str_cat_lit(mrb, out, "false");
  else if (mrb_string_p(value)) json_write_string(mrb, out, RSTRING_PTR(value), RSTRING_LEN(value));
  else if (mrb_symbol_p(value)) {
    mrb_int length;
    const char *name = mrb_sym_name_len(mrb, mrb_symbol(value), &length);
    json_write_string(mrb, out, name, length);
  }
  else if (mrb_float_p(value)) {
    mrb_float number = mrb_float(value);
    if (isnan(number) || isinf(number)) {
      mrb_raisef(mrb, json_error_class(mrb, "GeneratorError"), "%v not allowed in JSON", mrb_obj_as_string(mrb, value));
    }
    mrb_str_cat_str(mrb, out, mrb_obj_as_string(mrb, value));
  }
  else if (mrb_integer_p(value) || mrb_obj_is_kind_of(mrb, value, mrb_class_get(mrb, "Integer"))) {
    mrb_str_cat_str(mrb, out, mrb_obj_as_string(mrb, value));
  }
  else if (mrb_array_p(value)) {
    mrb_str_cat_lit(mrb, out, "[");
    mrb_int length = RARRAY_LEN(value);
    for (mrb_int i = 0; i < length; i++) {
      if (i > 0) mrb_str_cat_lit(mrb, out, ",");
      json_newline(mrb, out, indent, depth + 1);
      json_write(mrb, out, RARRAY_PTR(value)[i], indent, depth + 1);
    }
    if (length > 0) json_newline(mrb, out, indent, depth);
    mrb_str_cat_lit(mrb, out, "]");
  }
  else if (mrb_hash_p(value)) {
    mrb_value keys = mrb_hash_keys(mrb, value);
    mrb_int length = RARRAY_LEN(keys);
    mrb_str_cat_lit(mrb, out, "{");
    for (mrb_int i = 0; i < length; i++) {
      mrb_value key = RARRAY_PTR(keys)[i];
      if (i > 0) mrb_str_cat_lit(mrb, out, ",");
      json_newline(mrb, out, indent, depth + 1);
      mrb_value name = mrb_string_p(key) ? key : mrb_obj_as_string(mrb, key);
      json_write_string(mrb, out, RSTRING_PTR(name), RSTRING_LEN(name));
      if (mrb_nil_p(indent)) mrb_str_cat_lit(mrb, out, ":");
      else mrb_str_cat_lit(mrb, out, ": ");
      json_write(mrb, out, mrb_hash_get(mrb, value, key), indent, depth + 1);
    }
    if (length > 0) json_newline(mrb, out, indent, depth);
    mrb_str_cat_lit(mrb, out, "}");
  }
  else {
    mrb_value text = mrb_obj_as_string(mrb, value);
    json_write_string(mrb, out, RSTRING_PTR(text), RSTRING_LEN(text));
  }
  mrb_gc_arena_restore(mrb, arena_index);
}

static mrb_value
json_generate(mrb_state *mrb, mrb_value self)
{
  mrb_value value;
  mrb_value indent = mrb_nil_value();
  mrb_get_args(mrb, "o|o", &value, &indent);
  if (!mrb_string_p(indent)) indent = mrb_nil_value();
  mrb_value out = mrb_str_new_capa(mrb, 64);
  json_write(mrb, out, value, indent, 0);
  return out;
}

static mrb_value
tachyon_emit(mrb_state *mrb, mrb_value self)
{
  mrb_value payload;
  mrb_get_args(mrb, "S", &payload);
  tac_host_emit(RSTRING_PTR(payload), (size_t)RSTRING_LEN(payload));
  return mrb_nil_value();
}

static mrb_value
tachyon_write(mrb_state *mrb, mrb_value self)
{
  mrb_value text;
  mrb_get_args(mrb, "S", &text);
  fwrite(RSTRING_PTR(text), 1, (size_t)RSTRING_LEN(text), stdout);
  fflush(stdout);
  return mrb_nil_value();
}

static char *
tac_answer(const char *pointer, size_t length)
{
  char *answer = (char *)malloc(length + 1);
  if (!answer) return NULL;
  memcpy(answer, pointer, length);
  answer[length] = '\0';
  return answer;
}

static char *
tac_error_answer(mrb_state *mrb, mrb_value exception)
{
  mrb_value message = mrb_funcall_argv(mrb, exception, mrb_intern_lit(mrb, "message"), 0, NULL);
  if (mrb->exc || !mrb_string_p(message)) { mrb->exc = NULL; message = mrb_str_new_lit(mrb, "error"); }
  mrb_value text = mrb_str_new_cstr(mrb, mrb_obj_classname(mrb, exception));
  mrb_str_cat_lit(mrb, text, ": ");
  mrb_str_cat_str(mrb, text, message);
  mrb_value out = mrb_str_new_lit(mrb, "{\"error\":");
  json_write_string(mrb, out, RSTRING_PTR(text), RSTRING_LEN(text));
  mrb_str_cat_lit(mrb, out, "}");
  return tac_answer(RSTRING_PTR(out), RSTRING_LEN(out));
}

static mrb_state *
tac_state(mrb_bool open_if_missing)
{
  static mrb_state *state = NULL;
  if (!state && open_if_missing) state = mrb_open();
  return state;
}

typedef struct {
  const char *pointer;
  size_t length;
  int bytecode;
} tac_source;

static mrb_value
tac_boot_body(mrb_state *mrb, void *userdata)
{
  tac_source *source = (tac_source *)userdata;
  if (source->bytecode) return mrb_load_irep_buf(mrb, source->pointer, source->length);
  return mrb_load_nstring(mrb, source->pointer, source->length);
}

EMSCRIPTEN_KEEPALIVE
char *
tac_boot(const char *pointer, size_t length, int bytecode)
{
  mrb_state *mrb = tac_state(TRUE);
  if (!mrb) return tac_answer("{\"error\":\"mruby could not start\"}", 33);
  int arena_index = mrb_gc_arena_save(mrb);
  tac_source source = { pointer, length, bytecode };
  mrb_bool failed = FALSE;
  mrb_value result = mrb_protect_error(mrb, tac_boot_body, &source, &failed);
  if (!failed && mrb->exc) { failed = TRUE; result = mrb_obj_value(mrb->exc); }
  mrb->exc = NULL;
  char *answer = failed ? tac_error_answer(mrb, result) : NULL;
  mrb_gc_arena_restore(mrb, arena_index);
  return answer;
}

static mrb_value
tac_invoke_body(mrb_state *mrb, void *userdata)
{
  tac_source *source = (tac_source *)userdata;
  mrb_value engine = mrb_obj_value(mrb_module_get(mrb, "TachyonWeb"));
  mrb_value text = mrb_str_new(mrb, source->pointer, source->length);
  return mrb_funcall_argv(mrb, engine, mrb_intern_lit(mrb, "invoke"), 1, &text);
}

EMSCRIPTEN_KEEPALIVE
char *
tac_invoke(const char *pointer, size_t length)
{
  mrb_state *mrb = tac_state(FALSE);
  if (!mrb) return tac_answer("{\"error\":\"not booted\"}", 22);
  int arena_index = mrb_gc_arena_save(mrb);
  tac_source source = { pointer, length, 0 };
  mrb_bool failed = FALSE;
  mrb_value result = mrb_protect_error(mrb, tac_invoke_body, &source, &failed);
  mrb->exc = NULL;
  char *answer;
  if (failed) answer = tac_error_answer(mrb, result);
  else if (mrb_string_p(result)) answer = tac_answer(RSTRING_PTR(result), RSTRING_LEN(result));
  else answer = tac_answer("{\"error\":\"engine answered a non-string\"}", 40);
  mrb_gc_arena_restore(mrb, arena_index);
  return answer;
}

EMSCRIPTEN_KEEPALIVE
void *
tac_alloc(size_t size)
{
  return malloc(size);
}

EMSCRIPTEN_KEEPALIVE
void
tac_free(void *pointer)
{
  free(pointer);
}

void
mrb_tachyon_web_gem_init(mrb_state *mrb)
{
  struct RClass *json = mrb_define_module(mrb, "JSON");
  struct RClass *base = mrb_define_class_under(mrb, json, "JSONError", E_STANDARD_ERROR);
  struct RClass *parser_error = mrb_define_class_under(mrb, json, "ParserError", base);
  mrb_define_class_under(mrb, json, "NestingError", parser_error);
  mrb_define_class_under(mrb, json, "GeneratorError", base);
  mrb_define_module_function(mrb, json, "parse", json_parse, MRB_ARGS_ARG(1, 1));
  mrb_define_module_function(mrb, json, "generate", json_generate, MRB_ARGS_ARG(1, 1));
  struct RClass *web = mrb_define_module(mrb, "TachyonWeb");
  mrb_define_module_function(mrb, web, "host_write", tachyon_write, MRB_ARGS_REQ(1));
  mrb_define_module_function(mrb, web, "host_emit", tachyon_emit, MRB_ARGS_REQ(1));
}

void
mrb_tachyon_web_gem_final(mrb_state *mrb)
{
}
