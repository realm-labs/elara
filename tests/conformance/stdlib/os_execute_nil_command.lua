local default_available = os.execute()
local nil_available = os.execute(nil, "ignored", false)

return string.byte(type(nil_available), 1), nil_available == default_available
