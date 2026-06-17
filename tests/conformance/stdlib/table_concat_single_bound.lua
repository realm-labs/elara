local joined = table.concat({"a", "b", "c"}, "-", 2, 2)

return string.len(joined), string.byte(joined, 1)
