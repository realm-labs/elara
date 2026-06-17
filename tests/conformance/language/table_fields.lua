local t = {
  10,
  20,
  35,
  name = 30,
  [4] = 40,
}

t.extra = t[1] + t.name

return t[1], t[2], t.name, t[4], t.extra, rawlen(t)
