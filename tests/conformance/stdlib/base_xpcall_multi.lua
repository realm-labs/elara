local function worker()
  return 2, nil, 4
end

local function handler()
  return 9
end

return xpcall(worker, handler)
