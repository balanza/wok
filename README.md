<p align="center">
  <img width="200" height="200" alt="image" src="https://github.com/user-attachments/assets/6e44dedf-79b0-4119-9d5a-481ccc7512c5" />
</p>
<h1 align="center">Wok</h1>
<h3 align="center">A tool for organising and managing projects.</h3>


`wok` is a CLI that makes it easy to move around projects. You can think about it as a smart alias for shortcutting frequently used commands.

It is designed with the following principles in mind:
* **Type as few characters as possible**; `wok` alone will infer what to do for daily tasks (add project, go to project), add flags for once-in-a-while commands (setup, export/import), uses fuzzy search whenever possible, provides autocomplete when it makes sense.
* **Pwd independent**; `wok` commands run against the target workspace regardless of the current *pwd*.


## Quick start

```sh
# show workspace dashboard and status
# run setup if not already done
wok

# clone the repo into $WOK_SPACE/acme/foo
wok git@github.com:acme/foo.git

# change the directory to the project dir
wok acme/foo

# list all the projects in the workspace, grouped by org
wok -l 

# like list, but filtered orgs only
wok -l --org acme,ymca

# remove one project
wok --rm acme/foo

# fast-forward all main branches for all repos
wok --ff 

# like ff, but filtered orgs only
wok --ff --org acme,ymca

# run setup on the current shell (force re-run)
# if not provided, the shell will be detected automatically
# using --manual will print out instructions without modifying the current system
wok --setup
wok --setup --shell zsh
wok --setup --manual

# export the list of all projects
wok --export > wok.json

# add all project from a previous export
wok --import wok.json
cat wok.json | wok --import

# scan a directory for git repositories
wok --scrape /path/to/directory

# like scrape, but filtered orgs only
wok --scrape /path/to/directory --org acme,ymca

# export the list of discovered repositories
wok --scrape /path/to/directory --export > discovered.json

# import all discovered repositories into the workspace
wok --scrape /path/to/directory --import

# clean setup
wok --clean
```

## Installation

### Pre-built Binaries (Recommended)

Download the latest release for your platform from the [releases page](https://github.com/balanza/wok/releases):

### Quick Install Script

```bash
curl -sSL https://raw.githubusercontent.com/balanza/wok/main/install.sh | bash
```

### Build from Source

For development or if pre-built binaries don't work for your system:

#### Prerequisites
- Rust 1.70 or later

#### Steps

```sh
# Clone the repository
git clone https://github.com/balanza/wok.git
cd wok

# Build the project
cargo build --release

# The binary will be available at ./target/release/wok
./target/release/wok --help

# Optionally, install it to your system
cargo install --path .
```

## F.A.Q.

### How do I start?
Just `wok` and the workspace dashboard will tell you what to do. Most likely, you will need to setup if it's the first usage (the dashboard itself will prompt you, just type `y`).

If you want to change the workspace directory, set the `WOK_SPACE` environment variable.

### Why do I need to setup?
To allow seamless integration with the shell when `cd`-ing to the project's root, `wok` needs to be wrapped with the `.wokrc` shell script. 

Also, autocomplete features need dedicated installation to work, depending on the host's shell.

### How do I start using `wok` with my current projects
`wok` relies on projects to be under version control (supports `git` only for now) and stored in child directories of the target workspace. 

You can import from an existing system using the `scrape` command:
```sh
wok --scrape /path/to/your/projects/
```

`wok` will look recursively into all child directories and will prompt you with the list of projects to import.

### How to have multiple workspaces?
Workspace can be selected by the value of `WOK_SPACE` environment variable, and each workspace is considered isolated by `wok`.

For example, to have one workspace for work and one for personal stuff:

```bash
# set aliases
work() {
  WOK_SPACE=~/work wok "$@"
}
personal () {
  WOK_SPACE=~/personal wok "$@"
}

# use work and personal as work aliases
> work git@github.com:acme/foo.git
> personal git@github.com:balanza/balanza.github.io.git

> work -l 
acme
└── foo

> personal -l
balanza
└── balanza.github.io
```

## Future developments
* handle `git worktree` for projects
* cross-project tasks (example: fast-fetch from main branch)
* improve stats in dashboard
