local ok, message, extra = pcall(pcall)

return ok, string.byte(type(message), 1), extra == nil
