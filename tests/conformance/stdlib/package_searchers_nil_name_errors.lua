local preload_ok, preload_message = pcall(package.searchers[1], nil)
local lua_ok, lua_message = pcall(package.searchers[2], nil)
local c_ok, c_message = pcall(package.searchers[3], nil)
local c_root_ok, c_root_message = pcall(package.searchers[4], nil)

return preload_ok,
  string.byte(type(preload_message), 1),
  lua_ok,
  string.byte(type(lua_message), 1),
  c_ok,
  string.byte(type(c_message), 1),
  c_root_ok,
  string.byte(type(c_root_message), 1)
