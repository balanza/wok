# Plan: Worktree UX features for wok

## Context

Wok manages git projects as an opaque `<org>/<project>` tree under `$WOK_SPACE`
with worktrees stored at `<org>/.worktrees/<project>/<name>`. The worktree
feature set today covers list (`wok -w`), create+navigate (`wok -w <name>`),
root jump (`wok -W`), and remove (`wok -w <name> --rm`). This plan adds four
capabilities aimed at the daily loop of a coder working multiple lines of work:

1. **`--take` / `-T`** — move uncommitted changes along when switching
   worktrees, tracked *and* untracked files (via `git stash -u`; exploits the
   fact that `git stash` is repo-global, visible from every worktree).
2. **Root↔worktree toggle** — `wok -w -` / `wok -`, ping-pong to the last
   location wok moved us from (like `cd -`). Wrapper-side only.
3. **PR review as a dedicated feature** — not an overload of `-w`. `wok
   <pr-url>` fetches the PR into a worktree under the hood; if the worktree
   already exists, it pulls (fetch + merge; conflicts abort with an error).
   The URL may be any GitHub PR page subpath (`/commits`, `/files`, …) — wok
   reduces it to org, project, and PR number. If the project is not in the
   workspace, wok clones it first. `wok --pr <n>` does the same for a project
   resolved from the positional argument or the current directory. No `gh`
   dependency: plain git fetch, which works for fork PRs too (refs/pull lives
   on the base repo). GitHub-only ref namespace; documented as such.
4. **Richer worktree menu labels** — show branch and dirty state in the FD3
   selection menu.

Full rationale and tool inspiration (lazygit, git-town, `gh pr checkout
--worktree` PR, gh-pr-worktree, gh-glance, jj workspaces) are in
`docs/superpowers/specs/2026-10-03-worktree-ux-ideas.md`. Deliberately out of
scope for this pass: merged-worktree pruning (`wok --ff -w`), tmux sessions per
worktree, untracked-config bootstrap, `gh`-based PR autocomplete.

## Approach

- `--take` (short `-T`) is a new composable flag on the `-w` path
  (`wok -w <dest> -T`), following the `--rm` + `-w` precedent. Carry = stash
  `-u` in the *current* directory — which includes untracked files, but not
  gitignored ones — then pop in the *destination*. Both git calls are run
  by the wok binary itself using `git -C <dir>` — `git -C` points git at any
  directory, so wok can stash the directory the user is *leaving* and pop
  into the directory they are *entering*, in the same process invocation,
  strictly before writing the `goto::` FD3 marker. The wrapper's `cd` happens
  only after the binary exits, so there is no ordering or timing dependency:
  by the time the shell changes directory, the changes are already there.
  Safety: destination must be clean (a
  newly created worktree trivially is); a failed/conflicted pop preserves the
  stash by git's own semantics, so nothing can be lost; error if destination
  is dirty (kept open — see open question 1).
- The toggle never reaches the binary: the `wok()` shell function intercepts
  `-w -` / `--worktree -` / bare `wok -` and `cd`s to a remembered
  `__wok_last` variable; every successful cd performed by the wrapper (goto
  and menu selection) records the previous `$PWD`. Binary stays stateless and
  pwd-independent.
- PR review is a dedicated feature, not an overload of `-w`: the user says
  `wok <pr-url>` or `wok --pr <n>` (project from the positional argument or
  the cwd). Under the hood it is still a worktree at
  `<org>/.worktrees/<repo>/pr/<n>` with local branch `pr/<n>`, but a user of
  the PR flow never needs to know that. URL handling: any GitHub PR page
  subpath (`github.com/<org>/<repo>/pull/<n>/...`) is reduced to
  org/repo/number; `<org>/<repo>` resolves against the workspace and is
  cloned first when missing (reusing the `add` clone logic). Existing
  worktree → `fetch origin pull/<n>/head` + `merge FETCH_HEAD`, with
  `merge --abort` and a returned error on conflicts; dirty worktree prompts,
  and `--force` (explicit, destructive) resets hard to the PR head. New
  worktree → `fetch origin pull/<n>/head:refs/heads/pr/<n>` then the existing
  `create_worktree` pass-through. Removing a `pr/<n>` worktree additionally
  deletes the local branch.
- Menu labels extend the existing `label::path` mechanism: worktree entries
  become `<name> (<branch>) !` (trailing ` !` when dirty). Stdout listing
  (consumed by autocomplete) stays bare names.

## Files to modify

- `src/cli.rs` — `-T/--take` flag (requires `worktree` with a value,
  conflicts with `rm`), `take: bool` on `InferredCommand::GoWorktree`;
  `--pr <number>` and `--force` flags with new PR command variants; GitHub
  PR-URL inference ahead of `Add`; `cli_test!` / `cli_test_error!` tables for
  all of the above.
- `src/commands/pr.rs` (new; register in `src/commands/mod.rs`, dispatch in
  `src/main.rs`) — PR flow: URL reduction, workspace resolution,
  clone-if-missing, worktree create/sync, FD3 goto.
- `src/commands/go_worktree.rs` — `--take` carry flow; rich labels in
  `handle_list`.
- `src/lib/git.rs` — `WorktreeItem.branch` (from porcelain `branch` lines;
  detached HEAD fallback), `stash_all(path)` / `stash_pop(path)` helpers,
  `fetch_pr_head(...)` / `merge_ref(...)` / `reset_hard(...)` helpers,
  `delete_branch` helper for PR cleanup.
- `src/commands/remove_worktree.rs` — delete `pr/<n>` local branch after
  worktree removal.
- `templates/.wokrc.bash`, `templates/.wokrc.zsh` — toggle interception +
  `__wok_last` recording on every cd.
- `src/lib/shell.rs` — extend `compile_wokrc_script` tests to assert the new
  script content (status is content-compared, so `OutOfDate` detection needs
  no version bump).
- `tests/worktree.rs` — integration tests (local `git init`, network-free,
  plus a local bare remote exposing `refs/pull/1/head` for the PR flow).
- `tests/cli.rs` — only if new inference cases aren't covered by unit
  `cli_test!` tables (they should be).

## Reuse

- `wok::lib::git::is_worktree_dirty` — dirty checks for `--take`, PR sync, and
  labels (`src/lib/git.rs:190`).
- `wok::lib::git::create_worktree` — extra-args pass-through already supports
  checking out an existing branch (`git worktree add <dir> <branch>`).
- `wok::lib::git::current_branch` — root label and per-worktree branch label.
- `resolve_project_path` (`src/commands/go_worktree.rs:22`) — unchanged; it
  already handles arbitrary worktree nesting depth below
  `.worktrees/<project>/`.
- `MULTIPLE_MATCHES_MARKER` `label::path` entries — rich labels need no
  wrapper-format change (the wrapper already renders the label part only).
- `cli_test!` macro convention in `src/cli.rs` for all new inference cases.
- Test helpers `init_git_project` / `add_worktree` / `current_branch` in
  `tests/worktree.rs`.

## Tasks

Built strictly one task at a time: each task gets its own branch; when the
implementation is done you test and review it; after approval we merge and
only then start the next task. One feature per commit (conventional commits,
scoped `feat(...)`), author `Emanuele De Cupis <emanuele@decup.is>`.

- [ ] **Task 1 — `-T/--take` carry flag**
  - `src/cli.rs`: `-T/--take` (requires `-w` with a value, conflicts with
    `--rm`, rejected with menu-only `-w`), `take: bool` on `GoWorktree`,
    `cli_test!` / `cli_test_error!` cases.
  - `src/lib/git.rs`: `stash_all(path)` (`git stash push -u`) and
    `stash_pop(path)` helpers + unit tests.
  - `src/commands/go_worktree.rs`: carry in `handle_search` — destination
    clean or newly created → stash in cwd (silent no-op when clean) → pop in
    destination → goto; dirty destination → error; pop failure → error
    noting the stash survived.
  - `tests/worktree.rs`: tracked *and* untracked files move across worktrees;
    dirty-destination refusal.
- [ ] **Task 2 — root↔worktree toggle**
  - `templates/.wokrc.{bash,zsh}`: intercept `-w -`, `--worktree -`, bare
    `wok -` (cd to `__wok_last`, stderr error when unset); record
    `__wok_last=$PWD` before every cd (goto + menu selection).
  - `src/lib/shell.rs`: extend `compile_wokrc_script` assertions.
- [ ] **Task 3 — PR review feature (`wok <pr-url>` / `wok --pr <n>`)**
  - `src/cli.rs`: `--pr <number>` flag (project from positional or cwd) and
    `--force`; URL inference — any GitHub PR page subpath
    (`github.com/<org>/<repo>/pull/<n>/...`) routes to the PR flow instead of
    `Add`; `cli_test!` / `cli_test_error!` cases.
  - `src/commands/pr.rs` (new; register in `mod.rs`, dispatch in `main.rs`):
    reduce URL to org/repo/number; resolve `<org>/<repo>` in the workspace,
    clone when missing (reuse the `add` clone path); then ensure worktree
    `pr/<n>` — new: `fetch origin pull/<n>/head:refs/heads/pr/<n>` +
    `create_worktree` pass-through; existing: `fetch origin pull/<n>/head` +
    `merge FETCH_HEAD`, `merge --abort` + error on conflict; dirty → prompt,
    `--force` → `reset --hard` to the PR head; goto via FD3.
  - `src/commands/remove_worktree.rs` + `src/lib/git.rs`: removing a `pr/<n>`
    worktree also deletes the local `pr/<n>` branch; `delete_branch` helper.
  - `tests/worktree.rs`: network-free tests against a local bare remote
    exposing `refs/pull/1/head` (pushable with plain git): create, re-run
    sync, conflict abort, `--force`, clone-if-missing, `--rm` cleanup.
- [ ] **Task 4 — rich worktree menu labels**
  - `src/lib/git.rs`: `WorktreeItem.branch` from porcelain (`detached` when
    absent); update `parse_worktree_list` tests.
  - `src/commands/go_worktree.rs`: `handle_list` FD3 entries
    `<name> (<branch>) !` when dirty; stdout completion listing unchanged.
- [ ] **Task 5 — docs**
  - README: `-T/--take` (untracked included), toggle syntax, PR URL + `--pr`
    flow (GitHub-only caveat, `--force` semantics), labels.

## Verification

- Per task: full `cargo test` + `cargo build` (warnings-free, `cargo fmt`
  clean) on the task branch before it is handed to you for testing and
  review; merge only after your approval, then start the next task.
- `cargo test` — unit tables, `tests/worktree.rs` integration (network-free),
  `tests/cli.rs` unaffected unless new cases added there.
- Manual smoke for the toggle (template-only, not covered by `cargo test`):
  in a scratch shell, `source <compiled .wokrc.zsh>`, `wok <project>`, then
  `wok -w -` → back at previous dir; `wok -` with no history → stderr error;
  menu selection followed by `wok -` → returns to pre-menu dir.
- Manual smoke for PR flow against a real GitHub repo:
  `wok https://github.com/org/repo/pull/123/files` (subpath) with the
  project both present and missing (clone case) creates + jumps; a re-run
  after a PR push pulls; a locally-committed divergence conflicts → abort +
  error; `--force` resets; `wok <repo> -w pr/123 --rm` removes worktree and
  local branch.
- `cargo build` warnings-free; `cargo fmt` clean.

## Open questions

1. **`--take` with dirty destination** — deliberately kept open (your call):
   Task 1 ships with *error* (never merge); after you test it we decide
   whether an attempt-and-surface-conflicts mode is worth adding.
2. Resolved in your review: PR re-sync prompts when dirty, and `--force`
   (explicitly destructive) resets hard to the PR head.
3. Resolved in your review: the toggle ships as both `wok -w -` and bare
   `wok -`.
4. **Remote selection (clarifying your ❓)** — PR fetches go through the
   `origin` remote. In the URL flow a missing project is cloned, so `origin`
   comes from the PR URL itself; an existing project without an `origin`
   remote is an error with a hint rather than guessing another remote.
5. Resolved in your review: work proceeds one task at a time (branch →
   implement → you test and review → merge → next task).
