_wok_completion() {
  local cur="${COMP_WORDS[COMP_CWORD]}"
  local has_w=0
  local project_arg=""
  local i

  # Check if -w or --worktree is on the command line
  for (( i=1; i < COMP_CWORD; i++ )); do
    if [[ "${COMP_WORDS[$i]}" == "-w" || "${COMP_WORDS[$i]}" == "--worktree" ]]; then
      has_w=1
      break
    fi
  done

  if [[ $has_w -eq 1 ]]; then
    # Find the project argument (first positional arg, before -w)
    for (( j=1; j < i; j++ )); do
      if [[ "${COMP_WORDS[$j]}" != -* ]]; then
        project_arg="${COMP_WORDS[$j]}"
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
      COMPREPLY=($(compgen -W "$worktree_list" -- "$cur"))
    else
      COMPREPLY=()
      echo ""
      echo "no worktrees found (a new one will be created)"
    fi
    return
  fi

  local projects
  projects=$(wok --list --format flat 2>/dev/null | sed 's/ \*$//')

  if [[ -n "$projects" ]]; then
    COMPREPLY=($(compgen -W "$projects" -- "$cur"))
  fi
}

complete -F _wok_completion wok
