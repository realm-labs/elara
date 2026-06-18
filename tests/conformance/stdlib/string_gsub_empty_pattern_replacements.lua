local table_text, table_count = string.gsub("ab", "", { [""] = "." })

local function replace_empty(value)
  if value == "" then
    return "x"
  end
  return "?"
end

local function_text, function_count = string.gsub("ab", "", replace_empty)

return string.len(table_text), string.byte(table_text, 1),
  string.byte(table_text, 2), string.byte(table_text, 3),
  string.byte(table_text, 4), string.byte(table_text, 5), table_count,
  string.len(function_text), string.byte(function_text, 1),
  string.byte(function_text, 2), string.byte(function_text, 3),
  string.byte(function_text, 4), string.byte(function_text, 5), function_count
