local result, message, stage = package.loadlib("missing.so", "luaopen_missing", "ignored", false)

return rawequal(result, nil), string.byte(type(message), 1), string.byte(stage, 1), string.len(stage)
