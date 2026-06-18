local searcher_args = 0

local function loader(...)
  local name, data = ...
  return data + string.len(name)
end

local function searcher(...)
  searcher_args = select("#", ...)
  local name = ...
  return loader, string.len(name) + 60
end

package.searchers[1] = searcher
local first, data = require("custom.extra", "ignored", false)
local second = package.loaded["custom.extra"]

return first, data, second, searcher_args
