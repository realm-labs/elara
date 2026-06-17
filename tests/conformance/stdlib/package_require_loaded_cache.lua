package.loaded.cached = 123

local calls = 0

local function loader()
  return 99
end

local function searcher()
  calls = calls + 1
  return loader
end

package.searchers[1] = searcher

local first = require("cached")
local second = require("cached")

return first, second, calls
