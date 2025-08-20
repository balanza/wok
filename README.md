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

# setup on the current shell (first use only)
# the current shell is detected automatically
wok setup

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
