for key, value in pairs({ name = 41 }) do
  return string.byte(key, 1), #key, value
end

return 0, 0, 0
