# gitqlite: Implement Git with SQL

Use an SQLite database to replace the .git directory and implement all your git operations in SQL statements.

## Goals

 The goal of this project is to **have fun**. It tries to map out the core data structures of git (blob, tree, commit, and everything you would see under your `.git` directory) not as files in the filesystem but as tables in a SQLite database, and uses SQL to implement the "plumbing" commands of git that manipulates this database.
 On top of this whimsical layer, we implement a git-like interface supporting high-level operations like `add`, `status`, and `commit`.

There is an attempt to make the user interface layer git-like and familiar, but the underlying data structures are not git-compatible. (Though We haven't done it yet) We could well map out a tree into a bunch of rows in the tree table, each describing one entry, for example, and use a completely different hashing scheme as the objects are not deserialized into the git's file structure in the first place.

This project is one of my attempts to play with arbitrary "what-ifs" that pop into my mind from nowhere. It is not intended to be used in any serious way (you probably don't want to version control your big repository with gitqlite), but I do plan to do some benchmark to see exactly how bad it is.

## Design

The project maintains `.gitqlite` directory under your repository root and all the data structures are stored inside an SQLite database under this directory. When invoked, the command opens up the database and manipulates the state using plain SQL statements.

The configurations and git ignore files are two exceptions. They are git-compatible and gitqlite will automatically use your gitconfig settings (local, global, and system) and it will respect rules defined in .gitignore files.

## Build & Develop

Gitqlite is a standard rust project managed with cargo.

```shell
# Build the binary
cargo build

# Init current directory as a gitqlite repository - this will create the .gitqlite directory and initialize the SQLite database
cargo run -- init
```

