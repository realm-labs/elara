#ifndef ELARA_LAUXLIB_H
#define ELARA_LAUXLIB_H

#include "lua.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct luaL_Reg {
    const char *name;
    lua_CFunction func;
} luaL_Reg;

lua_State *luaL_newstate(void);
int luaL_loadbufferx(lua_State *L, const char *buff, size_t size, const char *name, const char *mode);
int luaL_loadstring(lua_State *L, const char *s);
int luaL_error(lua_State *L, const char *fmt, ...);
const char *luaL_checklstring(lua_State *L, int arg, size_t *len);
lua_Integer luaL_checkinteger(lua_State *L, int arg);
lua_Number luaL_checknumber(lua_State *L, int arg);
void luaL_checktype(lua_State *L, int arg, int t);
void luaL_openlibs(lua_State *L);

#define luaL_loadbuffer(L,s,sz,n) luaL_loadbufferx((L), (s), (sz), (n), NULL)

#ifdef __cplusplus
}
#endif

#endif
