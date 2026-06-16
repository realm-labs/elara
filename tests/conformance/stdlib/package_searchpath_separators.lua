local path, message = package.searchpath("a.b", "x/?.lua;y/?.lua", ".", "/")
local first = string.find(message, "x/a/b.lua", 1, true)
local second = string.find(message, "y/a/b.lua", 1, true)

return rawequal(path, nil), first ~= nil, second ~= nil
