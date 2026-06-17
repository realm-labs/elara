local function fallback()
  return 99
end

local values = setmetatable({}, {
  __index = fallback,
})

local written = rawset(values, "name", 42)

return rawequal(rawget(values, "missing"), nil),
  values.missing,
  rawequal(written, values),
  rawget(values, "name"),
  values.name
