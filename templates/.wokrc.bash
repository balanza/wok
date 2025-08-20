#!/bin/bash

wok() {
        local output
        output=$("__WOK_BINARY_PATH__" "$@" 2>&1)
        local retcode=$?
        if echo "$output" | grep --color=auto --exclude-dir={.bzr,CVS,.git,.hg,.svn,.idea,.tox,.venv,venv} -q '^__WOK_GOTO_MARKER__'; then
                local line
                line=$(echo "$output" | grep '^__WOK_GOTO_MARKER__' | head -n 1)
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
                echo "$output" | grep --color=auto --exclude-dir={.bzr,CVS,.git,.hg,.svn,.idea,.tox,.venv,venv} -v '^__WOK_GOTO_MARKER__'
        else
                echo "$output"
        fi
        return $retcode
}
