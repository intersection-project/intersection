...

## Examples

Everyone with both role `cool person` who isn't `staff`: `@{{ "cool person" - staff }}`

All online `mods`: `@{{ mods & here }}`

## Precedence

All operators are parsed left-to-right. You can use parenthesis to manually override this. `A & B & C` is parsed as `(A & B) & C`.

## Internals (for nerds)

You can learn more about how it all works: {cmd_about_how_it_works}
If you've got an interest in parsing algorithms, we'd love your help!
