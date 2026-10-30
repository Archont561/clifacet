#!/usr/bin/env bash
# Start a working session from any checkout: get to the remote head, restore the
# sandbox environment, install the Git hooks, and prove the result.
#
# This exists because a fresh workspace can arrive at an old commit with no
# `.pixi` environment: only tracked files travel, so the checkout and the
# restored environment are both things a session has to re-establish. Doing that
# by hand is where stale-HEAD and missing-toolchain sessions come from.
#
# The checkout repair is deliberately conservative. It fast-forwards only a
# clean tree that is strictly behind its upstream, and refuses everything else
# (uncommitted changes, local commits, a detached HEAD) with the exact command
# to run. Losing work to a bootstrap script would be worse than a stale tree.
set -euo pipefail


SKIP_RESTORE=0
FORCE_RESTORE=0
SKIP_HOOKS=0
SKIP_CHECKS=0
NO_FETCH=0

usage() {
  cat <<'EOF'
Usage: scripts/bootstrap.sh [options]

  --skip-restore   do not run the sandbox restore even when the environment is missing
  --force-restore  restore even when a Pixi environment is already present
  --skip-hooks     do not install the lefthook Git hooks
  --skip-checks    do not run the post-restore smoke checks
  --no-fetch       do not contact the remote; check the checkout as it is
  -h, --help       show this message
EOF
}

for argument in "$@"; do
  case "$argument" in
    --skip-restore) SKIP_RESTORE=1 ;;
    --force-restore) FORCE_RESTORE=1 ;;
    --skip-hooks) SKIP_HOOKS=1 ;;
    --skip-checks) SKIP_CHECKS=1 ;;
    --no-fetch) NO_FETCH=1 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "bootstrap: unknown option $argument" >&2; usage >&2; exit 2 ;;
  esac
done

say() { printf '%s\n' "$*"; }
problem() { printf 'bootstrap: %s\n' "$*" >&2; }

# --- 1. the checkout ---------------------------------------------------------

sync_checkout() {
  local branch upstream head target
  branch=$(git -C "$ROOT" symbolic-ref --quiet --short HEAD || true)
  if [ -z "$branch" ]; then
    problem "HEAD is detached; fix the branch yourself with 'git switch <branch>'"
    return 0
  fi
  if [ "$NO_FETCH" -eq 0 ]; then
    # A branch that exists only locally is normal for a session branch, so ask
    # before fetching: `git fetch` prints a bare `fatal:` for it, which reads
    # like a broken checkout rather than a branch nobody has pushed yet.
    if ! git -C "$ROOT" ls-remote --exit-code --quiet --heads origin "$branch" >/dev/null 2>&1; then
      say "checkout: $branch is not on origin yet; nothing to compare"
      return 0
    fi
    # An explicit refspec, because `git fetch origin <branch>` updates FETCH_HEAD
    # only: the comparison below reads refs/remotes/origin/<branch>.
    if ! git -C "$ROOT" fetch --quiet origin "+refs/heads/$branch:refs/remotes/origin/$branch"; then
      problem "cannot fetch origin/$branch; continuing with the checkout as it is"
      return 0
    fi
  fi
  upstream="origin/$branch"
  if ! git -C "$ROOT" rev-parse --verify --quiet "$upstream^{commit}" >/dev/null; then
    say "checkout: $branch has no upstream yet; nothing to compare"
    return 0
  fi
  head=$(git -C "$ROOT" rev-parse HEAD)
  target=$(git -C "$ROOT" rev-parse "$upstream")
  if [ "$head" = "$target" ]; then
    say "checkout: $branch is at $upstream ($(git -C "$ROOT" rev-parse --short HEAD))"
    return 0
  fi
  if ! git -C "$ROOT" merge-base --is-ancestor "$head" "$upstream"; then
    # Local commits that the remote does not have: the work is intentional, keep it.
    problem "$branch has commits that $upstream does not; leaving the checkout alone"
    return 0
  fi
  if [ -n "$(git -C "$ROOT" status --porcelain)" ]; then
    problem "the checkout is behind $upstream but the tree has uncommitted changes"
    problem "run 'git stash && git merge --ff-only $upstream && git stash pop' yourself"
    return 1
  fi
  git -C "$ROOT" merge --quiet --ff-only "$upstream"
  say "checkout: fast-forwarded $branch to $upstream ($(git -C "$ROOT" rev-parse --short HEAD))"
}

# --- 2. the environment ------------------------------------------------------

needs_restore() {
  [ "$FORCE_RESTORE" -eq 1 ] && return 0
  command -v pixi >/dev/null 2>&1 || return 0
  [ -x "$ROOT/.pixi/envs/default/bin/python" ] || return 0
  return 1
}

refresh_sandbox_branch() {
  # `restore.sh` fetches a sandbox branch only when the ref is missing, and it
  # fetches it without a force marker -- but every publish rewrites that branch,
  # so an existing ref is both stale and un-fast-forwardable. A restore would
  # then verify an old pack and silently produce an old environment. Refreshing
  # here, with the force marker, is what makes a restore mean "current".
  if ! git -C "$ROOT" ls-remote --exit-code --quiet --heads origin 'sandbox/*' >/dev/null 2>&1; then
    say "environment: no sandbox branch on origin to refresh"
    return 0
  fi
  if git -C "$ROOT" fetch --quiet --depth 1 origin \
      '+refs/heads/sandbox/*:refs/remotes/origin/sandbox/*'; then
    say "environment: refreshed the sandbox branch from origin"
  else
    problem "environment: could not refresh the sandbox branch; a restore would use a stale pack"
  fi
}

restore_environment() {
  if [ "$SKIP_RESTORE" -eq 1 ]; then
    say "environment: restore skipped on request"
    return 0
  fi
  if ! needs_restore; then
    say "environment: Pixi $(pixi --version) already restored"
    return 0
  fi
  if [ "$NO_FETCH" -eq 0 ]; then
    refresh_sandbox_branch
  fi
  say "environment: restoring the sandbox (this verifies every packed file)"
  if ! bash "$ROOT/scripts/restore.sh" "$@"; then
    problem "restore failed; see the message above before running any check"
    return 1
  fi
  export PATH="$HOME/.local/bin:$PATH"
}

# --- 3. the hooks and a smoke check -----------------------------------------

install_hooks() {
  if [ "$SKIP_HOOKS" -eq 1 ]; then
    say "hooks: skipped on request"
    return 0
  fi
  if [ ! -d "$ROOT/.git" ]; then
    say "hooks: not a Git checkout"
    return 0
  fi
  if pixi run --frozen --no-install --offline hooks-install >/dev/null 2>&1; then
    say "hooks: installed from lefthook.yml"
  else
    problem "hooks: could not install; run 'pixi run --frozen --no-install hooks-install'"
  fi
}

smoke_checks() {
  if [ "$SKIP_CHECKS" -eq 1 ]; then
    say "checks: skipped on request"
    return 0
  fi
  local task
  for task in test-scripts alint; do
    if pixi run --frozen --no-install --offline "$task" >/dev/null 2>&1; then
      say "checks: $task passed"
    else
      problem "checks: $task failed; the environment is up, the repository is not clean"
      return 1
    fi
  done
}

main() {
  # Resolved after the options are parsed, the same rule `restore.sh` uses: the
  # repository being bootstrapped is the one the current directory is in, so
  # running this from a subdirectory works and a copy of the script never
  # bootstraps a different checkout than the caller meant.
  if ! ROOT=$(git rev-parse --show-toplevel 2>/dev/null); then
    problem "run this from inside a Git checkout"
    exit 1
  fi
  say "bootstrap: $ROOT"
  sync_checkout
  restore_environment "$@"
  export PATH="$HOME/.local/bin:$PATH"
  install_hooks
  smoke_checks
  say ""
  say "ready: export PATH=\"\$HOME/.local/bin:\$PATH\" in a new shell, then work through"
  say "       'pixi run --frozen backlog -- task list'. Pre-commit lints, pre-push audits."
}

main "$@"
