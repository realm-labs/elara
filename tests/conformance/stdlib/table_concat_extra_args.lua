local joined = table.concat({"a", "b", "c"}, "-", 1, 2, "ignored")

return joined == "a-b"
