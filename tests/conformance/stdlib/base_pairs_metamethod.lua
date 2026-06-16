local subject = {}

local function custom_next(state, control)
  if rawequal(control, nil) then
    return 41, rawequal(state, subject)
  end

  return nil
end

local function custom_pairs(value)
  return custom_next, value, nil, nil
end

local installed = setmetatable(subject, {
  __pairs = custom_pairs,
})

for key, value in pairs(subject) do
  return key + 1, value, rawequal(installed, subject)
end

return 0, false, rawequal(installed, subject)
