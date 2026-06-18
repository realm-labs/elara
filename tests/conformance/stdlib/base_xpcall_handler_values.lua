local function worker()
  error("boom")
end

local function handler(message)
  return type(message), string.byte(type(message), 1), 7
end

local ok, kind, byte, extra = xpcall(worker, handler)

return ok, string.byte(kind, 1), byte, extra
