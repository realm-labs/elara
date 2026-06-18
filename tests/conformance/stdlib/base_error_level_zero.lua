local ok, message, extra = pcall(error, "boom", 0)

return ok, message == "boom", extra == nil
