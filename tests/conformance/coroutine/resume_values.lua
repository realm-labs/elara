local function answer(a, b)
  return a + b, "done", nil, b
end

local co = coroutine.create(answer)
local ok, sum, label, missing, last = coroutine.resume(co, 20, 22)

local function wrapped(a, b)
  return a * b, "wrapped"
end

local product, wrapped_label = coroutine.wrap(wrapped)(6, 7)
return ok, sum, string.len(label), string.byte(label, 1), missing, last,
  product, string.len(wrapped_label), string.byte(wrapped_label, 1)
