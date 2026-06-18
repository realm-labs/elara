local path = package.searchpath("Cargo", "./?.toml", nil, nil)

return string.len(path), string.byte(path, 1), string.byte(path, 3)
