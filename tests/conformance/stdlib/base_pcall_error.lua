local function fail()
  local _ = error("boom")
end

return pcall(fail)
