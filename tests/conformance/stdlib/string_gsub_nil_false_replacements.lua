local table_text, table_count = string.gsub("abc 123 def", "(%a+)", {
  abc = false,
})

local function keep_original(word)
  if word == "abc" then
    return false
  end
  return nil
end

local function_text, function_count = string.gsub("abc 123 def", "(%a+)", keep_original)

return
  #table_text, string.byte(table_text, 1), string.byte(table_text, 9), table_count,
  #function_text, string.byte(function_text, 1), string.byte(function_text, 9), function_count
