local registry = debug.getregistry()
registry.answer = 42
local again = debug.getregistry()

return again.answer, rawequal(registry, again)
