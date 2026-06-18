local function worker()
  error("boom")
end

local function handler(message)
  return type(message), string.find(message, "boom", 1, true) ~= nil
end

local ok, kind, has_message = xpcall(worker, handler)

return ok, string.byte(kind, 1), has_message
