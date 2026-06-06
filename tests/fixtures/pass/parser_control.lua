global<const> *
local x = 1 + 2 * 3
function t.a:f(y, ... rest)
  if y then
    return y
  else
    return rest
  end
end
