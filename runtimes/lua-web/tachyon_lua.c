#include "lua.h"
#include "lauxlib.h"
#include "lualib.h"
#include <emscripten.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

__attribute__((import_module("tachyon"), import_name("tac_host_emit")))
void tac_host_emit(const char *pointer, size_t length);

__attribute__((import_module("tachyon"), import_name("tac_host_ask")))
char *tac_host_ask(const char *pointer, size_t length);

static const luaL_Reg tac_libraries[] = {
  { LUA_GNAME, luaopen_base },
  { LUA_COLIBNAME, luaopen_coroutine },
  { LUA_TABLIBNAME, luaopen_table },
  { LUA_STRLIBNAME, luaopen_string },
  { LUA_MATHLIBNAME, luaopen_math },
  { LUA_UTF8LIBNAME, luaopen_utf8 },
  { NULL, NULL }
};

static const char *const tac_removed[] = { "dofile", "loadfile", NULL };

static int
tachyon_emit(lua_State *L)
{
  size_t length;
  const char *text = luaL_checklstring(L, 1, &length);
  tac_host_emit(text, length);
  return 0;
}

static int
tachyon_ask(lua_State *L)
{
  size_t length;
  const char *text = luaL_checklstring(L, 1, &length);
  char *answer = tac_host_ask(text, length);
  if (answer == NULL) {
    lua_pushnil(L);
    return 1;
  }
  lua_pushstring(L, answer);
  free(answer);
  return 1;
}

static const luaL_Reg tac_host[] = {
  { "emit", tachyon_emit },
  { "ask", tachyon_ask },
  { NULL, NULL }
};

static char *
tac_copy(const char *pointer, size_t length)
{
  char *copy = (char *)malloc(length + 1);
  if (copy == NULL) return NULL;
  memcpy(copy, pointer, length);
  copy[length] = '\0';
  return copy;
}

static char *
tac_error(const char *message)
{
  static const char hex[] = "0123456789abcdef";
  static const char head[] = "{\"error\":\"";
  char *answer = (char *)malloc(strlen(message) * 6 + sizeof(head) + 3);
  if (answer == NULL) return NULL;
  char *out = answer + (sizeof(head) - 1);
  memcpy(answer, head, sizeof(head) - 1);
  for (const unsigned char *cursor = (const unsigned char *)message; *cursor; cursor++) {
    if (*cursor == '"' || *cursor == '\\') *out++ = '\\';
    if (*cursor >= 0x20) { *out++ = (char)*cursor; continue; }
    memcpy(out, "\\u00", 4);
    out[4] = hex[*cursor >> 4];
    out[5] = hex[*cursor & 15];
    out += 6;
  }
  memcpy(out, "\"}", 3);
  return answer;
}

static char *
tac_failure(lua_State *L)
{
  const char *message = lua_tostring(L, -1);
  char *answer = tac_error(message ? message : "error object is not a string");
  lua_settop(L, 0);
  return answer;
}

static void
tac_open(lua_State *L)
{
  for (const luaL_Reg *library = tac_libraries; library->func; library++) {
    luaL_requiref(L, library->name, library->func, 1);
    lua_pop(L, 1);
  }
  for (const char *const *name = tac_removed; *name; name++) {
    lua_pushnil(L);
    lua_setglobal(L, *name);
  }
  luaL_newlib(L, tac_host);
  lua_setglobal(L, "TachyonHost");
}

static lua_State *
tac_state(int open_if_missing)
{
  static lua_State *state = NULL;
  if (state == NULL && open_if_missing) {
    state = luaL_newstate();
    if (state != NULL) tac_open(state);
  }
  return state;
}

EMSCRIPTEN_KEEPALIVE
char *
tac_boot(const char *pointer, size_t length)
{
  lua_State *L = tac_state(1);
  if (L == NULL) return tac_error("Lua could not start");
  if (luaL_loadbufferx(L, pointer, length, "=services", "t") != LUA_OK) return tac_failure(L);
  if (lua_pcall(L, 0, 0, 0) != LUA_OK) return tac_failure(L);
  return NULL;
}

static int
tac_invoke_body(lua_State *L)
{
  lua_getglobal(L, "TacServices");
  luaL_checktype(L, -1, LUA_TTABLE);
  lua_getfield(L, -1, "invoke");
  lua_pushvalue(L, 1);
  lua_call(L, 1, 1);
  if (!lua_isstring(L, -1)) return luaL_error(L, "the engine answered a non-string");
  return 1;
}

EMSCRIPTEN_KEEPALIVE
char *
tac_invoke(const char *pointer, size_t length)
{
  lua_State *L = tac_state(0);
  if (L == NULL) return tac_error("not booted");
  lua_pushcfunction(L, tac_invoke_body);
  lua_pushlstring(L, pointer, length);
  if (lua_pcall(L, 1, 1, 0) != LUA_OK) return tac_failure(L);
  size_t size;
  const char *answer = lua_tolstring(L, -1, &size);
  char *copy = tac_copy(answer, size);
  lua_settop(L, 0);
  return copy;
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
