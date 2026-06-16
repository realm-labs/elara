local function loader(name, data)
  return data + string.len(name)
end

local function searcher(name)
  return loader, string.len(name) + 60
end

package.searchers[1] = searcher
local first, data = require("custom.mod")
local second = package.loaded["custom.mod"]

return first, data, second
