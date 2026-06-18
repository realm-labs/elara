local registry = debug.getregistry("ignored", false)
registry.extra = 73
local again = debug.getregistry(nil)

return again.extra, rawequal(registry, again)
