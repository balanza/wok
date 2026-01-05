#compdef wok

_wok() {
  local -a projects
  local project_list

  project_list=$(wok --list --format flat 2>/dev/null)

  if [[ -n "$project_list" ]]; then
    projects=(${(f)"$(echo "$project_list" | sed 's/ \*$//')"})

    _describe 'project' projects
  fi
}

zstyle ':completion:*' matcher-list 'm:{a-zA-Z}={A-Za-z}' 'r:|[._-]=* r:|=*' 'l:|=* r:|=*'

_wok "$@"
