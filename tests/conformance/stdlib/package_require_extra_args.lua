package.loaded.cached_extra = 123

local calls = 0

local function loader()
  return 99
end

local function searcher()
  calls = calls + 1
  return loader
end

package.searchers[1] = searcher

local first = require("cached_extra", "ignored", false)
local second = package.require("cached_extra", "ignored")

return first, second, calls
