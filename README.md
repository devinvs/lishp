# `lishp`

A cross between a lisp interpreter and a shell.

### Usage

For basic shell usage, the syntax is the same as a shell such as bash or zsh:

```
ls
cat file.txt
cd ~/downloads
```

These commands are equivalent to the following s-expressions:

```
(ls)
(cat file.txt)
(cd ~/downloads)
```

You can compose commands together using s-expressions:

```
(echo hello (whoami))
```

Low level text aliases can be set with the alias function:

```
(alias ls ls --color)
```

This alias makes sure that every time the program sees `ls` it replaces it with
`ls --color`

Defining tree level substitutions can be defined with the def function:

```
(def pi 3.14)

(echo (* 2 pi))
```

Finally functions can be defined as follows:

```
(defun square (x) (* x x))

(map square '(1 2 3 4 5 6))
```

TODO: talk about stdlib/prelude stuff

### Future Plans

1. Pipes

```
(head -n 3 (pipe ls))
```

2. File Redirection

```
(ls ())
```

3. Selecting stdout, stderr

```
(stdout (ls))
(stderr (ls))
```
