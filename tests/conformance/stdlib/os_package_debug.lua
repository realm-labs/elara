local info = debug.getinfo(1, "S")
return os.difftime(10, 4), string.byte(type(package.path), 1), string.byte(type(package.searchers), 1), string.byte(type(info.what), 1)
