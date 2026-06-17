local joined = table.concat({"a", "b", "c"}, nil, 2, 3)

return string.len(joined), string.byte(joined, 1), string.byte(joined, 2)
