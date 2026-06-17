local calls = 0

local function loader()
  calls = calls + 1
  if calls == 1 then
    return false
  end
  return 88
end

package.preload.falsemod = loader

local first = require("falsemod")
local cached_false = package.loaded.falsemod
local second = require("falsemod")
local cached_second = package.loaded.falsemod

return first, cached_false, second, cached_second, calls
