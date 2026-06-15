local function loader()
  return 42
end

package.preload.direct = loader
local found = package.searchers[1]("direct")

return found()
