local ok, message, extra = pcall(nil)

return ok, string.byte(type(message), 1), extra == nil
