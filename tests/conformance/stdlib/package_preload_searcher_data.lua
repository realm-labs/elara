local function loader()
  return 42
end

package.preload.direct = loader
local found, data = package.searchers[1]("direct")

return found(), string.byte(data, 1), string.len(data)
