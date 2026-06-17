local function worker(first, second, third)
  return first, second, third
end

local function handler()
  return 9
end

return xpcall(worker, handler, 10, nil, 30)
