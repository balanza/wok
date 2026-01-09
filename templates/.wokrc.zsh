#!/bin/zsh

wok() {
  local fd3=$(mktemp "${TMPDIR:-/tmp}/wok_fd3_$$.XXXXXX")
  local goto_dir
  local multiple_matches=0

  "__WOK_BINARY_PATH__" "$@" 3> "$fd3"

  # Read fd3 content
  local fd3_content
  fd3_content=$(cat "$fd3")
  rm -f "$fd3" || true

  # Check for multiple matches marker
  if echo "$fd3_content" | grep -q '^__WOK_MULTIPLE_MATCHES_MARKER__'; then
    multiple_matches=1
  fi

  # Check for goto marker
  if echo "$fd3_content" | grep -q '^__WOK_GOTO_MARKER__'; then
    local line
    line=$(echo "$fd3_content" | grep '^__WOK_GOTO_MARKER__' | head -n 1)
    goto_dir="${line#__WOK_GOTO_MARKER__}"
  fi

  # Handle multiple matches - visual menu
  if [[ $multiple_matches -eq 1 ]]; then
    # Parse matches from fd3 (skip first line which is the marker)
    local -a matches
    matches=()
    while IFS= read -r line; do
      if [[ "$line" != __WOK_MULTIPLE_MATCHES_MARKER__* && -n "$line" ]]; then
        matches+=("$line")
      fi
    done <<< "$fd3_content"

    # Interactive selection menu
    local current_index=1
    local num_matches=${#matches[@]}

    # Function to display the menu
    _wok_display_menu() {
      local idx=$1
      local total=$2
      local is_first=$3

      # Clear previous menu (move cursor up and clear lines)
      if [[ "$is_first" != "first" ]]; then
        for ((i=0; i<total; i++)); do
          tput cuu1  # Move cursor up one line
        done
      fi

      # Display menu with current selection highlighted
      for ((i=1; i<=total; i++)); do
        tput el  # Clear line
        if [[ $i -eq $idx ]]; then
          # Highlight using reverse video
          echo -e "\e[7m${matches[$i]}\e[0m"
        else
          echo "${matches[$i]}"
        fi
      done
    }

    # Show initial menu
    _wok_display_menu $current_index $num_matches "first"

    # Read input character by character
    while true; do
      # Read single character
      read -rsk1 key

      # Handle different key inputs
      case "$key" in
        $'\t')  # TAB key
          current_index=$(( (current_index % num_matches) + 1 ))
          _wok_display_menu $current_index $num_matches
          ;;
        $'\x1b')  # Escape sequence (arrow keys)
          read -rsk2 key  # Read the rest of the escape sequence
          case "$key" in
            '[A')  # Up arrow
              current_index=$(( ((current_index - 2 + num_matches) % num_matches) + 1 ))
              _wok_display_menu $current_index $num_matches
              ;;
            '[B')  # Down arrow
              current_index=$(( (current_index % num_matches) + 1 ))
              _wok_display_menu $current_index $num_matches
              ;;
          esac
          ;;
        ''|$'\n'|$'\r')  # ENTER key (empty string, newline, or carriage return)
          echo ""  # Move to new line after selection
          wok "${matches[$current_index]}"
          return $?
          ;;
        $'\x03')  # Ctrl+C
          echo ""
          return 1
          ;;
      esac
    done
  fi

  # Handle goto
  if [[ -n "$goto_dir" ]]; then
    cd "$goto_dir" || return $?
  fi

  return 0
}
