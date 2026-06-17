local string_key = "name"
local numeric_key = 2
local truthy_key = true
local values = {
  [string_key] = 10,
  [numeric_key] = 20,
  [truthy_key] = 30,
}

values[string_key] = values[string_key] + 1

return values.name, values[2], values[true]
