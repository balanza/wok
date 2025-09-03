#!/bin/zsh

wok() {
  local fd3=$(mktemp --suffix=_wok_fd3_$$)
  local goto_dir

   "__WOK_BINARY_PATH__" "$@" 3> "$fd3"

   while IFS= read -r line; do
      # Send directory change commands to main shell
      if [[ "$line" =~ ^__WOK_GOTO_MARKER__(.*)$ ]]; then
        goto_dir="${match[1]}"
      fi
    done < "$fd3"

    rm -f "$fd3" || true

    if [[ -n "$goto_dir" ]]; then
      cd "$goto_dir" || return $?
    fi

    return 0
}
