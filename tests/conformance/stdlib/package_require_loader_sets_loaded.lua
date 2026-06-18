local calls = 0

local function loader(name, data)
  calls = calls + 1
  package.loaded[name] = data + 5
  return nil
end

local function searcher()
  return loader, 70
end

package.searchers[1] = searcher

local first, data = require("manual.loaded")
local second = require("manual.loaded")

return first, data, second, calls
