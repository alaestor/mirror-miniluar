This list is a stack. Items are taken from the top and deleted upon completion.

TODO
---

- Implement a byte-oriented Lua/Luau lexer and conservative whitespace/comment compaction. Preserve literal spelling and shebangs; reject unfinished literals. Test token-boundary hazards and raw bytes before adding transformations.
- Implement stdin and single-file CLI modes, explicit output, default sibling output, and overwrite protection. Test through the executable, including errors and stream separation.
- Add deterministic directory/batch discovery, recursive mode, output directories, and collision preflight. Reject destinations that alias other inputs, including filesystem aliases. Decide whether shell-expanded globs suffice rather than adding a glob dependency.
- Add comment/whitespace preservation and newline options with regression tests from ../fork-MiniLua/tests. Preserve line-comment terminators when whitespace is removed.
- Add literal shortening only where Lua 5.1–5.4 and Luau agree. Check runtime bytes, float/integer distinctions, overflow, negative zero, and dialect-sensitive escapes against interpreters.
- Design scope-aware local renaming using a parser/binding analysis, not token substitution. Cover closures, shadowing, repeat/until, loop variables, labels, attributes, Luau types and interpolation. Fail closed on unsupported syntax.
- Validate the complete CLI against the original regression suite's behavioral guarantees, audit dependencies and every unsafe instance, and finish Nix package/check outputs. No README until requested.
