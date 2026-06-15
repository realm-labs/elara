local first = package.loadlib("missing.so", "luaopen_missing")

return rawequal(first, nil)
