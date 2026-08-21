; Tree-sitter highlight queries for Lale

((token) @comment
  (#match? @comment "^(//|///)"))

((token) @string
  (#match? @string "^\""))
((token) @string.special
  (#match? @string.special "^<[^>]+>$"))

((token) @character
  (#match? @character "^'"))

((token) @number
  (#match? @number "^[0-9]"))

((token) @boolean
  (#match? @boolean "^(true|false)$"))

((token) @keyword
  (#any-of? @keyword
    "var" "fn" "type" "use" "unsafe" "if" "else" "loop"
    "return" "debug" "rewind" "export" "import" "copy"
    "assert" "private" "enum" "switch" "case" "nothing" "when" "match"
    "write" "warn" "alert" "read" "end" "exit" "has"
    "has value" "has no value" "value of" "value at" "pointer to" "move on" "missing code"
    "end if" "end loop" "end fn" "end type" "end enum" "end switch" "end when" "end match"
    "end test suite" "end test case" "test suite" "test case"
    "exit loop" "exit program" "fn signature" "unsafe cast"
    "as" "in" "over" "from" "to" "step" "returns"))

((token) @type.builtin
  (#match? @type.builtin "^(u8|i8|u16|i16|u32|i32|u64|i64|f16|f32|f64|str|bool|byte|char|pointer)$"))

((token) @type.builtin
  (#match? @type.builtin "^(vec2|vec3|vec4)$"))

((token) @operator
  (#match? @operator "^[-+*/%^~=<>!.]+$"))

((token) @operator
  (#match? @operator "^(and|or|xor|not|invert|dot|cross|bitwise and|bitwise or|bitwise xor|unsigned left shift|unsigned right shift|signed left shift|signed right shift)$"))

((token) @operator
  (#match? @operator "^[⋅÷⨯]+$"))

((token) @constant.builtin
  (#match? @constant.builtin "^#"))
