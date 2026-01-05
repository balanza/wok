#!/bin/bash

wok() {
	local fd3=$(mktemp --suffix=_wok_fd3_$$)
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

		echo "Multiple projects found. Use TAB or arrow keys to navigate, ENTER to select, Ctrl+C to cancel"
		echo ""

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
				if [ $i -eq $idx ]; then
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

	# Check for goto marker
	if echo "$fd3_content" | grep -q '^__WOK_GOTO_MARKER__'; then
		local line
		line=$(echo "$fd3_content" | grep '^__WOK_GOTO_MARKER__' | head -n 1)
		local target_dir=${line#__WOK_GOTO_MARKER__}
		if [ -d "$target_dir" ]; then
			cd "$target_dir" || {
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
