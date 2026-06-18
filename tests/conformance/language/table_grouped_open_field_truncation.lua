local function tail()
  return 20, 30
end

local function values(...)
  local t = { 10, (tail()), (...) }
  return rawlen(t), t[1], t[2], t[3], t[4] == nil
end

return values(40, 50)
