local ok, message = pcall(error)
local nil_ok, nil_message = pcall(error, nil)

return ok, message == nil, nil_ok, nil_message == nil
