local count = 0

for _ in string.gmatch("ab", "") do
  count = count + 1
end

return count
