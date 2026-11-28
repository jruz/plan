mod rust '.just/rust.just'
mod just '.just/just.just'
mod oxfmt '.just/oxfmt.just'
mod git '.just/git.just'
mod tools '.just/tools.just'

check:
    just just::fmt-check
    just oxfmt::fmt-check
    just rust::check

fix:
    just just::fmt-fix
    just oxfmt::fmt-fix
    just rust::fix

setup:
    direnv allow
    just git::hooks-setup

install:
    just rust::install

test:
    just rust::test

run *ARGS='':
    just rust::run {{ ARGS }}

[positional-arguments]
release +ARGS:
    nu scripts/release.nu "$@"
