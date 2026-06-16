local function answer()
  return 42
end

local co = coroutine.create(answer)

return coroutine.isyieldable(),
  coroutine.isyieldable(co),
  coroutine.status(co) == "suspended"
