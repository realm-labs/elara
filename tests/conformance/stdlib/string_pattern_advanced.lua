local balanced = string.match("a(b(c)d)e", "%b()")
local frontier = string.match("abc 123", "%f[%d]%d+")

return string.len(balanced), string.len(frontier)
