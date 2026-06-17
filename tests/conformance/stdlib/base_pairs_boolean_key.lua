for key, value in pairs({ [true] = 42 }) do
  return key, value
end

return false, 0
