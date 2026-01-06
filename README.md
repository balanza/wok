<p align="center">
  <img width="200" height="200" alt="image" src="https://github.com/user-attachments/assets/6e44dedf-79b0-4119-9d5a-481ccc7512c5" />
</p>
<h1 align="center">Wok</h1>
<h3 align="center">A tool for organising and managing projects.</h3>




## Usage

```sh
# every command is executed referring to the base dir $WOK_SPACE
# default: ~/Workspace
echo $WOK_SPACE

# show workspace dashboard (projects count, orgs count)
wok

# setup on the current shell (first use only)
# if not provided, the shell will be detectged automatically
wok --setup
wok --setup --shell zsh
wok --setup --manual

# clone the repo into $WOK_SPACE/acme/foo
wok git@github.com:acme/foo.git

# change the directory to the project dir
wok <org>/<prj>

# list all the projects in the workspace, grouped by org
wok -l 

# like list, but filtered orgs only
wok -l --org acme,ymca

# fast-forward all main branches for all repos
wok --ff 

# like ff, but filtered orgs only
wok --ff --org acme,ymca

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
```
