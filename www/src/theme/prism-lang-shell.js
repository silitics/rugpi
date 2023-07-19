Prism.languages.shell = {
  function: {
    pattern: /(docker|rugpi-ctrl|\.\/run-bakery|\bgit\b)/
  },
  constant: {
    pattern: /(true|false)/,
    alias: "keyword",
  },
  parameter: {
    pattern: /(\s+--?[^\s]+|<\w+[^>]+)/,
    alias: "variable",
  },
  punctuation: {
    pattern: /(\\)/,
  }
}