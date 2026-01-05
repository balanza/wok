_wok_completion() {
  local cur="${COMP_WORDS[COMP_CWORD]}"
  local projects

  projects=$(wok --list --format flat 2>/dev/null | sed 's/ \*$//')

  if [[ -n "$projects" ]]; then
    COMPREPLY=($(compgen -W "$projects" -- "$cur"))
  fi
}

complete -F _wok_completion wok
