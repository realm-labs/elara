local function handler(message)
  return type(message), string.byte(type(message), 1)
end

local ok, kind, byte, extra = xpcall(nil, handler)

return ok, string.byte(kind, 1), byte, extra == nil
