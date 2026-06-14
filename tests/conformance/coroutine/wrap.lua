local function answer()
  return 42
end

local wrapped = coroutine.wrap(answer)
return wrapped()
