#ifndef ELARA_LUALIB_H
#define ELARA_LUALIB_H

#include "lua.h"

#ifdef __cplusplus
extern "C" {
#endif

#define LUA_GNAME "_G"
#define LUA_COLIBNAME "coroutine"
#define LUA_TABLIBNAME "table"
#define LUA_IOLIBNAME "io"
#define LUA_OSLIBNAME "os"
#define LUA_STRLIBNAME "string"
#define LUA_UTF8LIBNAME "utf8"
#define LUA_MATHLIBNAME "math"
#define LUA_DBLIBNAME "debug"
#define LUA_LOADLIBNAME "package"

int luaopen_base(lua_State *L);
int luaopen_coroutine(lua_State *L);
int luaopen_table(lua_State *L);
int luaopen_io(lua_State *L);
int luaopen_os(lua_State *L);
int luaopen_string(lua_State *L);
int luaopen_utf8(lua_State *L);
int luaopen_math(lua_State *L);
int luaopen_debug(lua_State *L);
int luaopen_package(lua_State *L);

void luaL_openlibs(lua_State *L);

#ifdef __cplusplus
}
#endif

#endif
