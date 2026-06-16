local path_pos = string.find(package.path, "?.lua", 1, true)

return string.byte(type(package.path), 1),
  string.byte(type(package.cpath), 1),
  rawequal(package.path, package.cpath),
  rawequal(path_pos, nil)
