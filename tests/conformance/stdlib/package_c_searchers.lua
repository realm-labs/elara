package.cpath = "x/?.dll;y/?.dll"

local c_miss = package.searchers[3]("missing")
local root_miss = package.searchers[4]("root.child")

return string.byte(c_miss, 1), string.len(c_miss),
  string.byte(root_miss, 1), string.len(root_miss)
