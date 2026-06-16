local path = package.searchpath("Cargo", "./?.toml")

return string.len(path), string.byte(path, 1), string.byte(path, 3)
