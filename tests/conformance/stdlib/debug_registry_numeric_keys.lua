local registry = debug.getregistry()
local key = 1001

rawset(registry, key, "value")
local again = debug.getregistry()
local value = rawget(again, key)
rawset(again, key, nil)

return rawequal(registry, again),
  value == "value",
  rawget(registry, key) == nil
