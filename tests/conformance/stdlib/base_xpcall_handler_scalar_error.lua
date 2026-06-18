local function worker()
  error(42)
end

local function handler(message)
  return type(message), message
end

local ok, kind, value = xpcall(worker, handler)

return ok, string.byte(kind, 1), value
