<!--do doc --readme header-->
This crate is part of the [`zng`](https://github.com/zng-ui/zng?tab=readme-ov-file#crates) project.

Cargo extension for Zng project management. Create a new project from templates, collect localization strings, package
the application for distribution.

# Installation

```console
cargo install cargo-zng
```

# Usage

Commands overview:

```console
# cargo zng --help

Zng project manager.

Usage: cargo-zng.exe <COMMAND>

Commands:
  new   Initialize a new repository from a Zng template repository
  l10n  Localization text scraper
  res   Build resources
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## `new`

Initialize a new repository from a Zng template repository.

```console
# cargo zng new --help

Initialize a new repository from a Zng template repository

Usage: cargo zng new [OPTIONS] [VALUE]...

Arguments:
  [VALUE]...
          Set template values by position

          The first value for all templates is the app name.

          EXAMPLE

          cargo zng new "My App!" | creates a "my-app" project.

          cargo zng new "my_app"  | creates a "my_app" project.

Options:
  -t, --template <TEMPLATE>
          Zng template

          Can be `.git` URL or an `owner/repo` for a GitHub repository.

          Can also be an absolute path or `./path` to a local template directory.

          [default: zng-ui/zng-template]

  -s, --set [<SET>...]
          Set a template value

          Templates have a `.zng-template` file that defines the possible options.

  -k, --keys
          Show all possible values that can be set on the template

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

Zng project generator is very simple, it does not use any template engine, just Rust's string replace in UTF-8 text files only.
The replacement keys are compilable, so template designers can build/check their template like a normal Rust project.

Template keys encode the format they provide, these are the current supported key cases:

* t-key-t — kebab-case
* T-KEY-T — UPPER-KEBAB-CASE
* t_key_t — snake_case
* T_KEY_T — UPPER_SNAKE_CASE
* T-Key-T — Train-Case
* t.key.t — lower case
* T.KEY.T — UPPER CASE
* T.Key.T — Title Case
* ttKeyTt — camelCase
* TtKeyTt — PascalCase
* {{key}} — Unchanged.

The values for each format (except {{key}}) are cleaned of chars that do not match this pattern
`[ascii_alphabetic][ascii_alphanumeric|'-'|'_'|' '|]*`. The case and separator conversions are applied to this
cleaned value.

The actual keys are declared by the template in a `.zng-template` file in the root of the template repository, they
are ascii alphabetic with >=3 lowercase chars.

Call `cargo zng new --keys` to show help for the template keys.

The default template has 3 keys:

* `app` — The app name, the Cargo package and crate names are derived from it. Every template first key must be this one.
* `org` — Used in `zng::env::init` as the 'organization' value.
* `qualifier` — Used in `zng::env::init` as the 'qualifier' value.

For an example input `cargo zng new "My App!" "My Org"` the template code:

```rust
// file: src/t_app_t_init.rs

pub fn init_t_app_t() {
    println!("init t-app-t");
    zng::env::init("{{qualifier}}", "{{org}}", "{{app}}");
}
```

Generates:

```rust
// file: src/my_app_init.rs

pub fn init_my_app() {
    println!("init my-app");
    zng::env::init("", "My Org", "My App!");
}
```

See [zng-ui/zng-template] for an example of templates.

[zng-ui/zng-template]: https://github.com/zng-ui/zng-template

## `l10n`

Localization text scraper.

```console
# cargo zng l10n --help

Localization text scraper

See the docs for `l10n!` for more details about the expected format.

Usage: cargo zng l10n [OPTIONS] <INPUT> <OUTPUT>

Arguments:
  <INPUT>
          Rust files glob

  <OUTPUT>
          Lang resources dir

Options:
  -m, --macros <MACROS>
          Custom l10n macro names, comma separated

          [default: ]

      --pseudo <PSEUDO>
          Pseudo Base name, empty to disable

          [default: pseudo]

      --pseudo-m <PSEUDO_M>
          Pseudo Mirrored name, empty to disable

          [default: pseudo-mirr]

      --pseudo-w <PSEUDO_W>
          Pseudo Wide name, empty to disable

          [default: pseudo-wide]

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

Also see [`zng::l10n::l10n!`] docs for more details about the expected format.

[`zng::l10n::l10n!`]: https://zng-ui.github.io/doc/zng/l10n/macro.l10n.html#scrap-template

# `res`

Build resources

```console
# cargo zng res --help

Build resources

Builds resources SOURCE to TARGET, delegates `.zr-{tool}` files to `cargo-zng-res-{tool}`
executables and crates.

Usage: cargo zng res [OPTIONS] [SOURCE] [TARGET]

Arguments:
  [SOURCE]
          Resources source dir

          [default: res]

  [TARGET]
          Resources target dir

          This directory is wiped before each build.

          [default: target/res]

Options:
      --pack
          Copy all static files to the target dir

      --tool-dir <DIR>
          Search for `zng-res-{tool}` in this directory first

          [default: tools]

      --tools
          Prints help for all tools available

      --tool <TOOL>
          Prints the full help for a tool

      --tool-cache <TOOL_CACHE>
          Tools cache dir

          [default: target/res.cache]

      --recursion-limit <RECURSION_LIMIT>
          Number of build passes allowed before final

          [default: 32]

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

This subcommand can be used to build resources and package releases. It is very simple, you create
a resources directory tree as close as possible to the final resources structure, and place special
`.zr-{tool}` files on it that are calls to `cargo-zng-res-{tool}` crates or executables.

## Resource Build

The resource build follows these steps:

* The TARGET dir is wiped clean.
* The SOURCE dir is walked, matching directories are crated on TARGET, `.zr-*` tool requests are run.
* The TARGET dir is walked, any new `.zr-*` request generated by previous pass are run (request is removed after tool run).
  - This repeats until a pass does not find any `.zr-*` or the `--recursion_limit` is reached.
* Run all tools that requested `zng-res::on-final=` from a request that still exists.

## Tools

You can call `cargo zng res --list` to see help for all tools available. Tools are searched in this order:

* If a crate exists in `tools/cargo-zng-res-{tool}` executes it (with `--quiet` build).
* If a crate exists in `tools/cargo-zng-res` and it has a `src/bin/{tool}.rs` file executes it with `--bin {tool}`.
* If the tool is builtin, executes it.
* If a `cargo-zng-res-{tool}[.exe]` is installed in the same directory as the running `cargo-zng[.exe]`, executes it.

### Authoring Tools

Tools are configured using environment variables:

* `ZR_SOURCE_DIR` — Resources directory that is being build.
* `ZR_TARGET_DIR` — Target directory where resources are being built to.
* `ZR_CACHE_DIR` — Dir to use for intermediary data for the specific request. Keyed on the source dir, target dir, request file and request file content.
* `ZR_WORKSPACE_DIR` — Cargo workspace that contains the source dir. This is also the working dir (`current_dir`) set for the tool.
* `ZR_REQUEST` — Request file that called the tool.
* `ZR_REQUEST_DD` — Parent dir of the request file.
* `ZR_TARGET` — Target file implied by the request file name. That is, the request filename without `.zr-{tool}` and in the equivalent target subdirectory.
* `ZR_TARGET_DD` — Parent dir of thr target file.
* `ZR_FINAL` — Set to the args if the tool requested `zng-res::on-final={args}`.
* `ZR_HELP` — Print help text for `cargo zng res --list`. If this is set the other vars will not be set.

In a Cargo workspace the [`zng::env::about`] metadata is also extracted from the primary binary crate:

* `ZR_APP` — package.metadata.zng.about.app or package.name
* `ZR_ORG` — package.metadata.zng.about.org or the first package.authors
* `ZR_VERSION` — package.version
* `ZR_DESCRIPTION` — package.description
* `ZR_HOMEPAGE` — package.homepage
* `ZR_PKG_NAME` — package.name
* `ZR_PKG_AUTHORS` — package.authors
* `ZR_CRATE_NAME` — package.name in snake_case
* `ZR_QUALIFIER` — package.metadata.zng.about.qualifier

[`zng::env::about`]: https://zng-ui.github.io/doc/zng_env/struct.About.html

Tools can make requests to the resource builder by printing to stdout with prefix `zng-res::`.
Current supported requests:

* `zng-res::delegate` — Continue searching for a tool that can handle this request.
* `zng-res::warning={message}` — Prints the `{message}` as a warning.
* `zng-res::on-final={args}` — Subscribe to be called again with `ZR_FINAL={args}` after all tools have run.

If the tool fails the entire stderr is printed and the resource build fails.

A rebuild starts by removing the target dir and runs all tools again. If a tool task is potentially
slow is should cache results. The `ZNG_RES_CACHE` environment variable is set with a path to a directory 
where the tool can store intermediary files specific for this request. The cache dir is keyed to the 
`<SOURCE><TARGET><REQUEST>` and the request file content.

The tool working directory (`current_dir`) is always set to the Cargo workspace root. if the `<SOURCE>`
is not inside any Cargo project a warning is printed and the `<SOURCE>` is used as working directory.

### Builtin Tools

These are the builtin tools provided:

```console
# cargo zng res --tools

.zr-copy @ cargo-zng
  Copy the file or dir

.zr-glob @ cargo-zng
  Copy all matches in place

.zr-rp @ cargo-zng
  Replace ${VAR} occurrences in the content

.zr-sh @ cargo-zng
  Run a bash script

.zr-warn @ cargo-zng
  Print a warning message

.zr-fail @ cargo-zng
  Print an error message and fail the build

call 'cargo zng res --help tool' to read full help from a tool
```

The expanded help for each:

#### `.zr-copy`

```console
$ cargo run -p cargo-zng -- res --tool copy

  Copy the file or dir

  The request file:
    source/foo.txt.zr-copy
     | # comment
     | path/bar.txt

  Copies `path/bar.txt` to:
    target/foo.txt

  Paths are relative to the Cargo workspace root.
```

#### `.zr-glob`

```console
$ cargo run -p cargo-zng -- res --tool glob

.zr-glob @ cargo-zng
  Copy all matches in place

  The request file:
    source/l10n/fluent-files.zr-glob
     | # localization dir
     | l10n
     | # only Fluent files
     | **/*.ftl
     | # except test locales
     | !:**/pseudo*

  Copies all '.ftl' not in a *pseudo* path to:
    target/l10n/

  The first path pattern is required and defines the entries that
  will be copied, an initial pattern with '**' flattens the matches.
  The path is relative to the Cargo workspace root.

  The subsequent patterns are optional and filter each file or dir selected by
  the first pattern. The paths are relative to each match, if it is a file
  the filters apply to the file name only, if it is a dir the filters apply to
  the dir and descendants.

  The glob pattern syntax is:

      ? — matches any single character.
      * — matches any (possibly empty) sequence of characters.
     ** — matches the current directory and arbitrary subdirectories.
    [c] — matches any character inside the brackets.
  [a-z] — matches any characters in the Unicode sequence.
   [!b] — negates the brackets match.

  And in filter patterns only:

  !:pattern — negates the entire pattern.
```

#### `.zr-rp`

```console
$ cargo run -p cargo-zng -- res --tool rp

.zr-rp @ cargo-zng
  Replace ${VAR} occurrences in the content

  The request file:
    source/greetings.txt.zr-rp
     | Thanks for using ${ZR_APP}!

  Writes the text content with ZR_APP replaced:
    target/greetings.txt
    | Thanks for using Foo App!

  The parameters syntax is ${VAR[:[case]][?else]}:

  ${VAR}          — Replaces with the ENV var value, or fails if it is not set.
  ${VAR:<case>}   — Replaces with the ENV var value case converted.
  ${VAR:?<else>}  — If ENV is not set or is set empty uses 'else' instead.
  $${VAR}         — Escapes $, replaces with '${VAR}'.

  The :<case> functions are:

  :k — kebab-case
  :K — UPPER-KEBAB-CASE
  :s — snake_case
  :S — UPPER_SNAKE_CASE
  :l — lower case
  :U — UPPER CASE
  :T — Title Case
  :c — camelCase
  :P — PascalCase
  :Tr — Train-Case
  : — Unchanged

  The fallback(else) can have nested ${VAR} patterns.

  Variables:

  All env variables are available, metadata from the binary crate is also available:

  ZR_APP — package.metadata.zng.about.app or package.name
  ZR_ORG — package.metadata.zng.about.org or the first package.authors
  ZR_VERSION — package.version
  ZR_DESCRIPTION — package.description
  ZR_HOMEPAGE — package.homepage
  ZR_PKG_NAME — package.name
  ZR_PKG_AUTHORS — package.authors
  ZR_CRATE_NAME — package.name in snake_case
  ZR_QUALIFIER — package.metadata.zng.about.qualifier

  See `zng::env::about` for more details about metadata vars.
  See the cargo-zng crate docs for a full list of ZR vars.
```

#### `.zr-sh`

```console
$ cargo run -p cargo-zng -- res --tool sh

.zr-sh @ cargo-zng
  Run a bash script

  Script is configured using environment variables (like other tools):

  ZR_SOURCE_DIR — Resources directory that is being build.
  ZR_TARGET_DIR — Target directory where resources are being built to.
  ZR_CACHE_DIR — Dir to use for intermediary data for the specific request.
  ZR_WORKSPACE_DIR — Cargo workspace that contains source dir. Also the working dir.
  ZR_REQUEST — Request file that called the tool (.zr-sh).
  ZR_REQUEST_DD — Parent dir of the request file.
  ZR_TARGET — Target file implied by the request file name.
  ZR_TARGET_DD — Parent dir of the target file.

  ZR_FINAL — Set if the script previously printed `zng-res::on-final={args}`.

  In a Cargo workspace the `zng::env::about` metadata is also set:

  ZR_APP — package.metadata.zng.about.app or package.name
  ZR_ORG — package.metadata.zng.about.org or the first package.authors
  ZR_VERSION — package.version
  ZR_DESCRIPTION — package.description
  ZR_HOMEPAGE — package.homepage
  ZR_PKG_NAME — package.name
  ZR_PKG_AUTHORS — package.authors
  ZR_CRATE_NAME — package.name in snake_case
  ZR_QUALIFIER — package.metadata.zng.about.qualifier

  Script can make requests to the resource builder by printing to stdout.
  Current supported requests:

  zng-res::warning={msg} — Prints the `{msg}` as a warning after the script exits.
  zng-res::on-final={args} — Schedule second run with `ZR_FINAL={args}`, on final pass.

  If the script fails the entire stderr is printed and the resource build fails.

  Runs on $ZR_SH, $PROGRAMFILES/Git/bin/sh.exe or sh.
```

#### `.zr-shf`

```console
$ cargo run -p cargo-zng -- res --tool shf

.zr-shf @ target/debug/cargo-zng
  Run a bash script on the final pass
  
  Apart from running on final this tool behaves exactly like .zr-sh
```

### `.zr-warn`

```console
$ cargo run -p cargo-zng -- res --tool warn

.zr-warn @ cargo-zng
  Print a warning message

  You can combine this with '.zr-rp' tool

  The request file:
    source/warn.zr-warn.zr-rp
     | ${ZR_APP}!

  Prints a warning with the value of ZR_APP
```

### `.zr-fail`

```console
$ cargo run -p cargo-zng -- res --tool fail

.zr-fail @ cargo-zng
  Print an error message and fail the build

  The request file:
    some/dir/disallow.zr-fail.zr-rp
     | Don't copy ${ZR_REQUEST_DD} with a glob!

  Prints an error message and fails the build if copied
```