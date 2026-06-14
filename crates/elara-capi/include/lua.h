#ifndef ELARA_LUA_H
#define ELARA_LUA_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

#define LUA_VERSION_MAJOR "5"
#define LUA_VERSION_MINOR "5"
#define LUA_VERSION_RELEASE "0"
#define LUA_VERSION_NUM 505
#define LUA_VERSION_RELEASE_NUM 50500
#define LUA_VERSION "Lua 5.5"
#define LUA_RELEASE "Lua 5.5.0"
#define LUA_SIGNATURE "\x1bLua"

#define LUA_MULTRET (-1)

#define LUA_OK 0
#define LUA_YIELD 1
#define LUA_ERRRUN 2
#define LUA_ERRSYNTAX 3
#define LUA_ERRMEM 4
#define LUA_ERRERR 5

#define LUA_TNONE (-1)
#define LUA_TNIL 0
#define LUA_TBOOLEAN 1
#define LUA_TLIGHTUSERDATA 2
#define LUA_TNUMBER 3
#define LUA_TSTRING 4
#define LUA_TTABLE 5
#define LUA_TFUNCTION 6
#define LUA_TUSERDATA 7
#define LUA_TTHREAD 8

#define LUA_MINSTACK 20
#define LUA_REGISTRYINDEX (-1000000)

typedef struct lua_State lua_State;
typedef long long lua_Integer;
typedef unsigned long long lua_Unsigned;
typedef double lua_Number;
typedef int (*lua_CFunction)(lua_State *L);
typedef ptrdiff_t lua_KContext;
typedef int (*lua_KFunction)(lua_State *L, int status, lua_KContext ctx);
typedef const char *(*lua_Reader)(lua_State *L, void *data, size_t *size);
typedef int (*lua_Writer)(lua_State *L, const void *p, size_t size, void *data);
typedef void *(*lua_Alloc)(void *ud, void *ptr, size_t osize, size_t nsize);
typedef void (*lua_WarnFunction)(void *ud, const char *msg, int tocont);

lua_State *lua_newstate(lua_Alloc f, void *ud);
void lua_close(lua_State *L);
lua_State *lua_newthread(lua_State *L);

int lua_gettop(lua_State *L);
void lua_settop(lua_State *L, int idx);
void lua_pushvalue(lua_State *L, int idx);
void lua_rotate(lua_State *L, int idx, int n);
void lua_copy(lua_State *L, int fromidx, int toidx);
int lua_checkstack(lua_State *L, int n);

int lua_type(lua_State *L, int idx);
const char *lua_typename(lua_State *L, int tp);
int lua_isnumber(lua_State *L, int idx);
int lua_isstring(lua_State *L, int idx);
int lua_iscfunction(lua_State *L, int idx);
int lua_isinteger(lua_State *L, int idx);

void lua_pushnil(lua_State *L);
void lua_pushnumber(lua_State *L, lua_Number n);
void lua_pushinteger(lua_State *L, lua_Integer n);
const char *lua_pushlstring(lua_State *L, const char *s, size_t len);
const char *lua_pushstring(lua_State *L, const char *s);
void lua_pushcclosure(lua_State *L, lua_CFunction fn, int n);
void lua_pushboolean(lua_State *L, int b);
void lua_pushlightuserdata(lua_State *L, void *p);

lua_Number lua_tonumberx(lua_State *L, int idx, int *isnum);
lua_Integer lua_tointegerx(lua_State *L, int idx, int *isnum);
int lua_toboolean(lua_State *L, int idx);
const char *lua_tolstring(lua_State *L, int idx, size_t *len);
lua_CFunction lua_tocfunction(lua_State *L, int idx);
void *lua_touserdata(lua_State *L, int idx);
const void *lua_topointer(lua_State *L, int idx);

int lua_load(lua_State *L, lua_Reader reader, void *data, const char *chunkname, const char *mode);
int lua_pcallk(lua_State *L, int nargs, int nresults, int msgh, lua_KContext ctx, lua_KFunction k);
int lua_dump(lua_State *L, lua_Writer writer, void *data, int strip);

#define lua_pop(L,n) lua_settop((L), -(n)-1)
#define lua_newtable(L) lua_createtable((L), 0, 0)
#define lua_pushcfunction(L,fn) lua_pushcclosure((L), (fn), 0)
#define lua_tonumber(L,i) lua_tonumberx((L), (i), NULL)
#define lua_tointeger(L,i) lua_tointegerx((L), (i), NULL)
#define lua_pcall(L,n,r,f) lua_pcallk((L), (n), (r), (f), 0, NULL)

void lua_createtable(lua_State *L, int narr, int nrec);

#ifdef __cplusplus
}
#endif

#endif
