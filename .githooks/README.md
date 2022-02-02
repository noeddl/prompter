This folder contains a pre-commit hook for automatically running `clippy` and `rustfmt` before each commit.

To make Git see the hooks in this folder run the following command.

```
$ git config core.hooksPath .githooks
```
