local result, message = package.searchpath(
  "missing",
  "x/?.lua;y/?.lua"
)

return rawequal(result, nil), string.byte(type(message), 1)
