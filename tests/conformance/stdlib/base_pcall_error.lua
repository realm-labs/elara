local function fail()
  local _ = error("boom")
end

local ok, message = pcall(fail)

return ok, string.byte(type(message), 1)
