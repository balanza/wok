# Ideas for wok worktree UX

Pre-design proposal for discussion. Not yet an approved spec — nothing here is
implemented. Each idea is grounded in the current wok codebase, inspired by
named tools, and shaped to wok's design principles: context-aware single
positional argument, composable flags (`--rm` removes a repo, `--rm` + `-w`
removes a worktree), opaque on-disk layout, autocomplete-first ergonomics,
and directory changes only via FD3 markers (`goto::`, `multiple::`).

## Current state (what exists today)

- `wok -w` — interactive FD3 menu of the resolved project's worktrees; root
  checkout shows as `* <branch>`; selecting it cd's to the root.
- `wok -w <name>` — exact-match navigation; if the name is unknown, prompts to
  create a worktree at `<org>/.worktrees/<project>/<name>` (branch implicitly
  named after the worktree, via `git worktree add <path>`); `-y` skips the
  prompt; extra args pass through to `git worktree add`.
- `wok -W [project]` — jump to the project's root checkout.
- `wok -w <name> --rm` / `--rm <project> -w` — remove a worktree (prompts when
  dirty, `-y` forces).
- Autocomplete completes worktree names for the resolved project.

## Idea 1 — Lines of work: richer worktree listing

**Gap:** the FD3 menu labels are bare worktree names; you can't see branch,
dirty state, or purpose without entering the worktree.

**Inspiration:**

- **lazygit** (jesseduffield/lazygit, PR #2147): a dedicated Worktrees tab
  showing branch and dirty status per worktree; treats worktree↔branch as a
  1:1 identity.
- **git-town** (branch types: `hack`, `observe`, `contribute`, `park`,
  `prototype`): attaches a *purpose* to each branch so sync/cleanup behavior
  differs per line of work.

**Proposals:**

1. **Richer menu labels** — extend `label::path` entries to
   `name (branch, ✱dirty)`. Pure win inside the existing label mechanism; no
   new flags; the shell wrapper already renders labels.
2. **Merged-worktree pruning** — compose the existing fast-forward flag with
   worktree: `wok --ff -w` fast-forwards and then reports worktrees whose
   branch is fully merged upstream, offering to remove them (mirrors git-town
   `sync` + prune-branches, but as flag composition, not a subcommand).

## Idea 2 — Root↔worktree toggle: `wok -w -`

**Gap:** switching between root and worktree always requires naming the
target; there is no "last place" memory.

**Inspiration:**

- **`cd -` / zoxide**: muscle memory built on *toggle*, not lookup.
- **lazygit** PR #4964: users asked to remove even one confirmation when
  switching between a root checkout and a worktree — toggle friction matters.

**Proposal:** `wok -w -` (possibly also bare `wok -`) = "take me to the last
place wok moved me from". Ping-pong between root and worktree, like `cd -`.
Implemented in the `~/.wokrc` wrapper template as a shell variable
(`$__wok_last`), keeping the Rust binary stateless and pwd-independent. The
wrapper, not the binary, remembers the previous location.

## Idea 3 — PR-review worktrees

**Gap:** `wok -w pr-123` would create a worktree+branch named `pr-123` off
your HEAD, but it never *fetches the PR*. The gap is the fetch, idempotent
re-sync, and cleanup.

**Inspiration:**

- **`gh pr checkout --worktree`** (cli/cli PR #13946, still open/unmerged):
  checks out a PR into a new worktree; idempotent on re-run (re-syncs instead
  of failing); handles forked PRs.
- **gh-pr-worktree** (brooke-hamilton, gh extension): worktrees per PR,
  fork support, removes the temporary remote when the worktree is removed.
- **gh-glance** (dora1998, gh extension): same pattern, optimized for the
  "PR needs testing but my working dir is dirty" moment.

**Proposal:** context-aware inference on the existing positional argument —
when `-w` is present and the term matches `pr/<n>` (or `pr-<n>`, or a bare
number):

1. `git fetch origin pull/<n>/head:pr/<n>` (works on GitHub without the `gh`
   binary; delegate to `gh pr checkout <n> --worktree` semantics when `gh` is
   available, for fork support);
2. create worktree `pr/<n>` at the usual opaque path;
3. jump into it (FD3 goto).

Idempotency: re-running `wok -w pr/<n>` when the worktree exists should
fetch + hard-reset to the PR head instead of erroring — PRs get force-pushed.
Cleanup: composing with the existing flag, `wok -w pr/<n> --rm` removes the
worktree *and* the local `pr/<n>` branch and remote-tracking ref
(gh-pr-worktree's cleanup lesson).

Autocomplete enrichment: complete `pr/<n>` from open PRs (`gh pr list`) with
the PR title — discovery, not just names.

## Idea 4 — Moving uncommitted changes: `--take`

**Key fact:** `git stash` lives in the shared object database (refs/stash) and
is visible from *every* worktree. `git stash -u` in worktree A, then
`git stash pop` in worktree B, works natively — most people don't know this.

**Inspiration:**

- **jj (Jujutsu)** `workspace` + `jj move`/`jj squash`: the working copy is a
  first-class commit, so moving uncommitted changes is just moving a commit
  between revisions. The philosophy — don't make the user think about
  committed vs uncommitted — is the north star.
- Native git: `git diff | git -C <other> apply` for tracked-only changes.

**Proposals:**

1. **Carry flag (recommended):** `wok -w <dest> --take` — if the current dir
   is dirty, `git stash -u` there, goto dest, `git stash pop`. Safety rules:
   pop only if the destination is clean (a failed/conflicted pop preserves
   the stash, so nothing can be lost); `--take` with a dirty destination is
   an error, not a merge.
2. **Smart default prompt:** when navigating away from a dirty worktree via
   the FD3 menu, offer "take changes with you? [y/N]". More magical, but puts
   a prompt on the hot path — better to start explicit and match `--rm`'s
   prompt philosophy (`-y` to skip).

## Idea 5 — Extras (beyond the four core areas)

1. **Editor/shell session per worktree** — inspired by **tmux-sessionizer**
   (ThePrimeagen `tms`) and **zellij** layouts: spawn/attach a tmux session
   named `wok:<project>/<worktree>` when entering a worktree. Trade-off:
   breaks the "just `cd`, shell-native" purity — if kept at all, opt-in via a
   `--setup` question; the wrapper template changes, the binary stays
   untouched.
2. **Untracked config bootstrap** — worktrees don't inherit untracked files
   (`.env`, `.envrc`, local settings); teams symlink by hand. Proposal: a
   per-project list of paths to copy/symlink into new worktrees, applied on
   create. Tension with the "opaque structure" principle (where does the
   list live?); flagged for discussion.

## Deliberately not proposed

Branch hierarchies / stacked PRs (**git-town**, **Graphite `gt`**,
**git-machete**): heavy machinery around push/sync semantics, orthogonal to
wok's "manage local checkouts" scope. `gt`'s only transferable lesson is the
naming discipline (worktree name = branch name), which wok already gets
implicitly from `git worktree add`.

## Suggested priority

1. Idea 4.1 (`--take`) and Idea 2 (`wok -w -`) — small, large ergonomic
   payoff, no network dependencies.
2. Idea 3 (PR worktrees) — biggest feature value, needs GitHub-specific
   logic.
3. Idea 1.1 (richer labels) — cosmetic, trivial.
4. Idea 1.2, Idea 5.1, Idea 5.2 — later discussion.

---

## Questions for the reviewer

:::question
Which ideas do you want to keep for a first design pass — and which should be
cut now?

The suggested priority is: `--take` (4.1), root↔worktree toggle (2), PR
worktrees (3), then the cosmetic/extras. Cut anything you don't see yourself
using.
:::

:::question
For `--take`, is a dirty destination an error (staying safe, my default) or
should wok attempt a stash-pop anyway and surface conflicts on stderr?

A failed or conflicted `git stash pop` preserves the stash, so nothing is
lost either way — this is purely about how often you want to be stopped.
:::

:::question
Should the tmux-session-per-worktree idea (5.1) be pursued at all, even as an
opt-in at setup time?

It conflicts with wok's shell-native `cd` model; answering "no" here closes it
for good.
:::
