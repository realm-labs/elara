local function loader()
  return 77
end

package.preload.mod = loader
local first = require("mod")
local function other()
  return 99
end
package.preload.mod = other
local second = require("mod")

package.path = "./?.lua"
local miss_len = string.len(package.searchers[2]("missing"))

return first, second, package.loaded.mod, miss_len
