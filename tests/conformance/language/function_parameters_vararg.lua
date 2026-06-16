local function collect(first, ... rest)
  return first, rest[1], rest[2], rest[3]
end

return collect(5, 6, 7)
