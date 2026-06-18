local path = package.searchpath("Cargo", "./?.toml", ".", "/", "ignored")

return string.len(path), string.byte(path, 1), string.byte(path, 3)
