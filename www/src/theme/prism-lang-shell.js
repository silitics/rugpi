Prism.languages.shell = {
  comment: {
    pattern: /#.*/,
  },
  function: {
    pattern: /(\sxz\s|\bwget\b|\bchmod\b|\bcurl\b|docker|ssh|scp|rugix-ctrl|jq|rugix-ctrl|rugix-bundler|^\.\/run-bakery|\bgit\b|\bdocker\b|\becho\b)/
  },  
  constant: {
    pattern: /(true|false|\b<[^>]*>\b|\bif\b|\bthen\b|\bfi\b|\belse\b)/,
    alias: "keyword",
  },
  parameter: {
    pattern: /(\s+--?[^\s]+|<\w+[^>]+)/,
    alias: "variable",
  },
  punctuation: {
    pattern: /(\\)|;/,
  },
  string: /"[^"]+"/,
}