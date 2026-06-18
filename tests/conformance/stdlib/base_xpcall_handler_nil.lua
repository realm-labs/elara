local function worker()
  error("boom")
end

local function handler()
  return nil
end

local ok, handled, extra = xpcall(worker, handler)

return ok, handled == nil, extra == nil
