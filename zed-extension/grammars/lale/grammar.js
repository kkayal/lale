// Minimal token-level grammar for Lale syntax highlighting.
// Lale has no reserved keywords, making a full parser impractical.
// This grammar only matches comment/string/number/keyword patterns for highlighting.
// All semantic features come from the LSP server.

module.exports = grammar({
  name: 'lale',

  extras: $ => [
    $._ws,
  ],

  rules: {
    source_file: $ => repeat($.token),

    _ws: $ => /[\s\n\r]+/,

    token: $ => token(choice(
      // Comments (must be first — match whole line)
      seq('///', /[^\n]*/),
      seq('//', /[^\n]*/),
      // Strings
      seq('"', repeat(choice(
        /[^"\\{]+/,
        seq('\\', /./),
        seq('{', /[^}]*/, '}'),
      )), '"'),
      // Character literals
      seq("'", /./, "'"),
      // Numbers (hex, float, int)
      /0[xX][0-9a-fA-F_]+/,
      /[0-9][0-9_]*\.[0-9]+([eE][+-]?[0-9]+)?/,
      /[0-9]+[eE][+-]?[0-9]+/,
      /[0-9]+/,
      // Multi-word keywords (longest first)
      seq('bitwise', /[\s\n\r]+/, choice('and', 'or', 'xor')),
      seq('unsigned', /[\s\n\r]+/, choice('left', 'right'), /[\s\n\r]+/, 'shift'),
      seq('signed', /[\s\n\r]+/, choice('left', 'right'), /[\s\n\r]+/, 'shift'),
      'pointer to',
      'value of',
      'value at',
      'has value',
      'has no value',
      'move on',
      'missing code',
      'end if',
      'end loop',
      'end fn',
      'end type',
      'end enum',
      'end match',
      'end switch',
      'end when',
      'exit loop',
      'exit program',
      'fn signature',
      'unsafe bitcast',
      'end test suite',
      'end test case',
      'test suite',
      'test case',
      // Compiler constants
      '#compiler_version',
      '#source_file',
      '#source_line',
      '#compile_time',
      '#function_name',
      '#posix',
      '#windows',
      '#debug',
      // Introspection
      '#type of',
      '#unit of',
      '#size of',
      // Single-word keywords
      'return', 'rewind', 'debug', 'unsafe',
      'export', 'import', 'copy',
      'assert', 'private', 'enum', 'case',
      'nothing',
      'write', 'warn', 'alert', 'read',
      'end', 'exit', 'has',
      'default', 'match', 'switch', 'when',
      // Types
      choice('vec2', 'vec3', 'vec4'),
      'u8', 'i8', 'u16', 'i16', 'u32', 'i32', 'u64', 'i64',
      'f16', 'f32', 'f64',
      'str', 'bool', 'byte', 'char', 'pointer',
      // Operators
      '+', '-', '*', '/', '%', '^', '~',
      '==', '!=', '≠', '>=', '<=', '≥', '≤', '>', '<',
      '+=', '-=', '*=', '/=', '%=', '=',
      'and', 'or', 'xor', '⊻', 'not', 'invert',
      '⋅', '÷', '⨯', 'dot', 'cross',
      'as', 'of', 'in', 'over', 'from', 'to', 'step',
      // Punctuation
      '(', ')', '[', ']', '{', '}',
      ',', '.', '->', ':', ';',
      // Keywords (context-dependent, but highlight them)
      choice('var', 'fn', 'type', 'use', 'loop', 'if', 'else'),
      // Identifiers
      /[a-zA-Z_\u00C0-\uFFFF][a-zA-Z0-9_\u00C0-\uFFFF]*/,
      // Units (single-line only, must not cross newlines)
      seq('<', /[^>\x0A\x0D]*/, '>'),
      // Anything else (catch-all)
      /./,
    )),
  },
});
