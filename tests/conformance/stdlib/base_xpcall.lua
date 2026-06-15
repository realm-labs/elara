local function handler()
  return 9
end

return xpcall(error, handler, "boom")
