local function pair()
  return 20, 30
end

local function values(...)
  local t = { pair(), ..., 40 }
  return rawlen(t), t[1], t[2], t[3]
end

return values(50, 60)
