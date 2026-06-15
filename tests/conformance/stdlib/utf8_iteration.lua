local total = 0
local last = 0

for position, codepoint in utf8.codes("ABC") do
  total = total + codepoint
  last = position
end

return total, last, utf8.offset("ABC", 2), utf8.offset("ABC", -1),
  string.byte(utf8.charpattern, 1)
