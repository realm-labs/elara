local function collect(first, ... rest)
  return first, rest[1], rest[2], rest.n
end

return collect(5, 6, 7)
