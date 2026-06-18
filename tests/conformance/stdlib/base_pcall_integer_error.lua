local ok, message, extra = pcall(error, 42)

return ok, message, extra == nil
