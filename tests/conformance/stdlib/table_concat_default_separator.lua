local joined = table.concat({"a", "b", "c"})

return string.len(joined), string.byte(joined, 1),
  string.byte(joined, 2), string.byte(joined, 3)
