local result, message, stage = package.loadlib("missing.so", "luaopen_missing")

return rawequal(result, nil), string.byte(type(message), 1), string.byte(stage, 1), string.len(stage)
