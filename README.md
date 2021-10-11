# PrettyPipe

Does one thing, and does it ~well~ poorly.

It will run a command for you and print its stdout and stderr in different colours (green and red).

```
$ prettypipe cat file1.txt file2.txt
<prints both files, in green>
$ prettypipe curl https://google.com
<prints the page's content in green and curl's progress info in red>
```

## Installation

Look, I don't know how to Rust.

```
$ cargo build --release
$ cp target/release/prettypipe /path/to/some/folder/on/your/PATH
```
