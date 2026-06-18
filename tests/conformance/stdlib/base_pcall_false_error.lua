local ok, message, extra = pcall(error, false)

return ok, message == false, extra == nil
