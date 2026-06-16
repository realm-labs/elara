local function loader()
  return 80
end

local function searcher()
  return loader, 70
end

package.searchers[1] = searcher
local first, data = require("custom.mod")
local second = package.loaded["custom.mod"]

return first, data, second
