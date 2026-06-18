local function target()
  return 42
end

local dumped = string.dump(target)

return string.byte(type(string.dump), 1),
  string.byte(type(dumped), 1),
  #dumped > 4
