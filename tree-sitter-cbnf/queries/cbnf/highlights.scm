(terminal) @string.grammar

(integer) @number

(comment) @comment

(identifier) @variable

(builtin 
  (identifier) @constant.builtin) @constant.builtin

(syntax_rule
  name: (identifier) @variable)

"=>>" @keyword.operator
"<<=" @keyword.operator

"(" @punctuation.bracket
")" @punctuation.bracket

"nil" @keyword
"except" @keyword
