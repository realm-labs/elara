local search_name_ok, search_name_message = pcall(package.searchpath, nil, "?.lua")
local search_path_ok, search_path_message = pcall(package.searchpath, "mod", nil)
local require_ok, require_message = pcall(require, nil)
local package_require_ok, package_require_message = pcall(package.require, nil)
local loadlib_path_ok, loadlib_path_message = pcall(package.loadlib, nil, "luaopen_missing")
local loadlib_init_ok, loadlib_init_message = pcall(package.loadlib, "missing.so", nil)

return search_name_ok,
  string.byte(type(search_name_message), 1),
  search_path_ok,
  string.byte(type(search_path_message), 1),
  require_ok,
  string.byte(type(require_message), 1),
  package_require_ok,
  string.byte(type(package_require_message), 1),
  loadlib_path_ok,
  string.byte(type(loadlib_path_message), 1),
  loadlib_init_ok,
  string.byte(type(loadlib_init_message), 1)
