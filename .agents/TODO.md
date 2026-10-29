This list is a stack. Items are taken from the top and deleted upon completion.

TODO
---

- Design scope-aware local renaming using a parser/binding analysis, not token substitution. Cover closures, shadowing, repeat/until, loop variables, labels, attributes, Luau types and interpolation. Fail closed on unsupported syntax.
- Validate the complete CLI against the original regression suite's behavioral guarantees, including runtime tests on Lua 5.1–5.3 and Luau. Only Lua 5.4 has been execution-tested so far. Audit dependencies and every unsafe instance. No README until requested.
