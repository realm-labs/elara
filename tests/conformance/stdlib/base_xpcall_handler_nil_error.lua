local function worker()
  error("outer")
end

local function handler()
  error(nil)
end

local ok, message, extra = xpcall(worker, handler)

return ok, message == nil, extra == nil
