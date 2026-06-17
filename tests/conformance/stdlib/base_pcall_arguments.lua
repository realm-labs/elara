local function worker(first, second, third)
  return first, second, third
end

return pcall(worker, 10, nil, 30)
