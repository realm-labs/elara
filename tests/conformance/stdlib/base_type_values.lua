local function f()
end

return string.byte(type(nil), 1),
  string.byte(type(false), 1),
  string.byte(type(1), 1),
  string.byte(type("x"), 1),
  string.byte(type({}), 1),
  string.byte(type(f), 1)
