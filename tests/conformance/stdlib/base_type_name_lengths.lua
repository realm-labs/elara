local function f()
end

return string.len(type(nil)),
  string.len(type(false)),
  string.len(type(1)),
  string.len(type("x")),
  string.len(type({})),
  string.len(type(f))
