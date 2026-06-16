local values = table.pack(package.path, package.path)
local joined = table.concat(values, "|")

return string.len(joined), string.byte(joined, 1), string.byte(joined, 126)
