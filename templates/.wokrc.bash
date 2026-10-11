#!/bin/bash

# cd that remembers the directory we leave in __wok_last, so the wrapper
# can toggle back to it (`wok -` / `wok -w -`).
_wok_cd() {
	local prev="$PWD"
	if cd "$1"; then
		if [ "$prev" != "$1" ]; then
			__wok_last="$prev"
		fi
		return 0
	fi
	return $?
}

wok() {
	# Root<->worktree toggle: `wok -` (bare) or `wok -w -` / `wok --worktree -`
	# jump to the last location wok moved us from, like `cd -`. Handled
	# entirely in the wrapper: the binary never sees the toggle and stays
	# stateless and pwd-independent.
	local wok_args=("$@")
	local wok_toggle=0
	if [ $# -eq 1 ] && [ "$1" = "-" ]; then
		wok_toggle=1
	else
		local i
		for (( i=0; i<$#; i++ )); do
			if [[ "${wok_args[$i]}" == "-w" || "${wok_args[$i]}" == "--worktree" ]] && (( i+1 < $# )) && [[ "${wok_args[$((i+1))]}" == "-" ]]; then
				wok_toggle=1
				break
			fi
		done
	fi
	if [ $wok_toggle -eq 1 ]; then
		if [ -n "${__wok_last:-}" ] && [ -d "${__wok_last}" ]; then
			local target="$__wok_last"
			__wok_last="$PWD"
			cd "$target" || return $?
		else
			echo "wok: no previous wok location to toggle back to" >&2
			return 1
		fi
		return 0
	fi

	local fd3=$(mktemp "${TMPDIR:-/tmp}/wok_fd3_$$.XXXXXX")
	local output
	output=$("__WOK_BINARY_PATH__" "$@" 3> "$fd3" 2>&1)
	local retcode=$?

	# Check fd3 for markers
	local fd3_content
	fd3_content=$(cat "$fd3")
	rm -f "$fd3" || true

	# Check for multiple matches marker
	if echo "$fd3_content" | grep -q '^__WOK_MULTIPLE_MATCHES_MARKER__'; then
		# Parse the matches from fd3 (skip first line which is the marker)
		local matches=()
		while IFS= read -r line; do
			if [[ "$line" != __WOK_MULTIPLE_MATCHES_MARKER__* && -n "$line" ]]; then
				matches+=("$line")
			fi
		done <<< "$fd3_content"

		# Interactive selection menu
		local current_index=0
		local num_matches=${#matches[@]}

		# Function to display the menu
		_wok_display_menu() {
			local idx=$1
			local total=$2
			# Clear previous menu (move cursor up and clear lines)
			if [ "$3" != "first" ]; then
				tput cuu $((total))  # Move cursor up
			fi
			for i in "${!matches[@]}"; do
				tput el  # Clear line
				# Show label part if entry has label::path format, otherwise show as-is
				local display="${matches[$i]}"
				if [[ "$display" == *"::"* ]]; then
					display="${display%%::*}"
				fi
				if [ $i -eq $idx ]; then
					# Highlight using reverse video
					echo -e "\e[7m${display}\e[0m"
				else
					echo "${display}"
				fi
			done
		}

		# Show initial menu
		_wok_display_menu $current_index $num_matches "first"

		# Read input character by character
		while true; do
			# Read single character
			IFS= read -rsn1 key

			# Handle different key inputs
			case "$key" in
				$'\t')  # TAB key
					current_index=$(( (current_index + 1) % num_matches ))
					_wok_display_menu $current_index $num_matches
					;;
				$'\x1b')  # Escape sequence (arrow keys)
					read -rsn2 key  # Read the rest of the escape sequence
					case "$key" in
						'[A')  # Up arrow
							current_index=$(( (current_index - 1 + num_matches) % num_matches ))
							_wok_display_menu $current_index $num_matches
							;;
						'[B')  # Down arrow
							current_index=$(( (current_index + 1) % num_matches ))
							_wok_display_menu $current_index $num_matches
							;;
					esac
					;;
				''|$'\n')  # ENTER key (empty string or newline)
					echo ""  # Move to new line after selection
					local selected="${matches[$current_index]}"
					if [[ "$selected" == *"::"* ]]; then
						_wok_cd "${selected#*::}" || return $?
					elif [[ "$selected" == /* ]]; then
						_wok_cd "$selected" || return $?
					else
						wok "$selected"
					fi
					return $?
					;;
				$'\x03')  # Ctrl+C
					echo ""
					return 1
					;;
			esac
		done
	fi

	# Check for goto marker
	if echo "$fd3_content" | grep -q '^__WOK_GOTO_MARKER__'; then
		local line
		line=$(echo "$fd3_content" | grep '^__WOK_GOTO_MARKER__' | head -n 1)
		local target_dir=${line#__WOK_GOTO_MARKER__}
		if [ -d "$target_dir" ]; then
			_wok_cd "$target_dir" || {
				echo "wok: failed to change directory to $target_dir" >&2
				return 1
			}
		else
			echo "wok: directory $target_dir does not exist" >&2
			return 1
		fi
	fi

	# Show output (without markers)
	echo "$output"
	return $retcode
}
