local function worker()
  return 1, nil, 3
end

return pcall(worker)
