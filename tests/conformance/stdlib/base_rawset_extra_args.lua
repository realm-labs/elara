local values = {}
local written = rawset(values, "name", 42, "ignored")

return rawequal(written, values),
  rawget(values, "name"),
  rawget(values, "ignored") == nil
