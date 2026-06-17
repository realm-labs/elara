package.loaded.reload = false

local calls = 0

local function loader()
  return 42
end

local function searcher()
  calls = calls + 1
  return loader
end

package.searchers[1] = searcher

local loaded = require("reload")

return loaded, package.loaded.reload, calls
