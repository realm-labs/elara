local key, value = next({ name = 41 })

return string.byte(key, 1), #key, value
