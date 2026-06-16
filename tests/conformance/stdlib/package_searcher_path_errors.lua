package.path = false
local lua_path_ok, lua_path_message = pcall(package.searchers[2], "mod")

package.cpath = false
local c_path_ok, c_path_message = pcall(package.searchers[3], "mod")
local root_path_ok, root_path_message = pcall(package.searchers[4], "mod.child")

return lua_path_ok,
  string.byte(type(lua_path_message), 1),
  c_path_ok,
  string.byte(type(c_path_message), 1),
  root_path_ok,
  string.byte(type(root_path_message), 1)
