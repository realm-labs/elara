local ok, message, extra = pcall(error, nil)

return ok, message == nil, extra == nil
