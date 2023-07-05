# TODO

- Pipes and file redirection
    + for pipes use either `pipe` or `|` as a function,
      maybe taking variable arguments (which we don't yet support?)
    + file redirection will probably just use `>` and `>>` etc..

- Variable argument lenths
  + use a `...` in the function args definition to define a list which is
    automatically created from variable arguments. Must be last arg

- Better autocomplete
  + Try to hook into zsh/bash completions packages so that we can autocomplete
    anything... Also maybe try to autocomplete arguments in path

- Better input
  + auto highlighting of parentheses
  + maybe move away from readline...
     + completion is just annoying to implement, at least with rustyline
