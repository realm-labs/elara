local require_type_ok, require_type_message = pcall(require, false)
local package_require_type_ok, package_require_type_message = pcall(package.require, false)

package.searchers = { false }
local searcher_type_ok, searcher_type_message = pcall(require, "broken")

return require_type_ok,
  string.byte(type(require_type_message), 1),
  package_require_type_ok,
  string.byte(type(package_require_type_message), 1),
  searcher_type_ok,
  string.byte(type(searcher_type_message), 1)
