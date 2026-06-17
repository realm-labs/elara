local joined = table.concat({ 1, 20, 3 }, "|")

return string.len(joined), string.byte(joined, 1),
  string.byte(joined, 2), string.byte(joined, 6)
