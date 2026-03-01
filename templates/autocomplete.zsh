#compdef wok

_wok() {
  local -a projects
  local -a worktrees
  local w_idx project_arg

  # Check if -w or --worktree is on the command line
  w_idx=${words[(I)-w]}
  if (( ! w_idx )); then
    w_idx=${words[(I)--worktree]}
  fi

  if (( w_idx )); then
    # Find the project argument (first positional arg, before -w)
    project_arg=""
    for (( i=2; i < w_idx; i++ )); do
      if [[ "${words[$i]}" != -* ]]; then
        project_arg="${words[$i]}"
        break
      fi
    done

    # Complete worktree names
    local worktree_list
    if [[ -n "$project_arg" ]]; then
      worktree_list=$("__WOK_BINARY_PATH__" "$project_arg" -w 2>/dev/null)
    else
      worktree_list=$("__WOK_BINARY_PATH__" -w 2>/dev/null)
    fi

    if [[ -n "$worktree_list" ]]; then
      worktrees=(${(f)worktree_list})
      _describe 'worktree' worktrees
    else
      _message 'no worktrees found (a new one will be created)'
    fi
    return
  fi

  project_list=$(wok --list --format flat 2>/dev/null)

  if [[ -n "$project_list" ]]; then
    projects=(${(f)"$(echo "$project_list" | sed 's/ \*$//')"})

    _describe 'project' projects
  fi
}

zstyle ':completion:*' matcher-list 'm:{a-zA-Z}={A-Za-z}' 'r:|[._-]=* r:|=*' 'l:|=* r:|=*'

_wok "$@"
