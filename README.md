# Wok
A tool for organizing and managing projects.

## Usage

```sh
# every command is executed referring to the base dir $WOK_SPACE
# default: ~/Workspace
echo $WOK_SPACE

# clone the repo into $WOK_SPACE/acme/foo
wok add git@github.com:acme/foo.git

# list all the projects in the workspace, grouped by org
wok list 

# like list, but filtered orgs only
wok list --org acme,ymca

# fast-forward all main branches for all repos
wok ff 

# like ff, but filtered orgs only
wok ff --org acme,ymca

# change the directory to the project dir
wok go <org>/<prj>

# export the list of all projects
wok export > wok.json

# add all project from a previous export
wok import wok.json
cat wok.json | wok import 
```
