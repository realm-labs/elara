local calls = 0

local function loader()
  return 99
end

local function miss_false()
  calls = calls + 1
  return false
end

local function miss_nil()
  calls = calls + 1
  return nil
end

local function found()
  calls = calls + 1
  return loader, 44
end

package.searchers[1] = miss_false
package.searchers[2] = miss_nil
package.searchers[3] = found

local module, data = require("skip.nonstring")

return module, data, calls
