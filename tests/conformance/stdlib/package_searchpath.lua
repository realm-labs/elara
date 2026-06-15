local path, message = package.searchpath(
  "missing",
  "x/?.lua;y/?.lua"
)

return rawequal(path, nil), rawequal(message, nil)
