Prism.languages.shell = {
  function: {
    pattern: /(\sxz\s|\bwget\b|\bcurl\b|docker|rugpi-ctrl|\.\/run-bakery|\bgit\b|\bdocker\b)/
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