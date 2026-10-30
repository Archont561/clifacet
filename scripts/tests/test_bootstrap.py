"""Bootstrap repairs only what it can repair without losing work.

Every case here drives the real script against a local bare remote, with the
restore, hooks and smoke checks switched off: the checkout policy is the part
that decides whether a session starts on the remote head or on a stale tree.
"""

import os
import subprocess
from pathlib import Path

BOOTSTRAP = Path(__file__).resolve().parents[1] / "bootstrap.sh"
QUIET = ["--skip-restore", "--skip-hooks", "--skip-checks"]


def git(cwd, *arguments):
    return subprocess.run(
        ["git", *arguments], cwd=cwd, text=True, capture_output=True, check=True
    ).stdout


def run(root, *arguments):
    return subprocess.run(
        ["bash", str(BOOTSTRAP), *QUIET, *arguments],
        cwd=root,
        text=True,
        capture_output=True,
        check=False,
        env={**os.environ, "HOME": str(root)},
    )


def build_repo(tmp_path):
    """A working clone whose upstream has one commit the clone does not."""
    remote = tmp_path / "remote.git"
    git(tmp_path, "init", "--bare", "-q", str(remote))
    # Commit before cloning: a clone of an empty remote has no branch to check out.
    seed = tmp_path / "seed"
    git(tmp_path, "init", "-q", str(seed))
    git(seed, "config", "user.email", "test@example.com")
    git(seed, "config", "user.name", "Test")
    (seed / "file.txt").write_text("first\n", encoding="utf-8")
    git(seed, "add", "file.txt")
    git(seed, "commit", "-q", "-m", "feat: first")
    git(seed, "remote", "add", "origin", str(remote))
    git(seed, "push", "-q", "origin", "HEAD:main")
    git(seed, "branch", "-q", "-M", "main")
    git(seed, "branch", "-q", "--set-upstream-to=origin/main", "main")
    # The bare remote's HEAD decides which branch a clone checks out.
    git(remote, "symbolic-ref", "HEAD", "refs/heads/main")

    clone = tmp_path / "clone"
    git(tmp_path, "clone", "-q", str(remote), str(clone))
    git(clone, "config", "user.email", "test@example.com")
    git(clone, "config", "user.name", "Test")

    (seed / "file.txt").write_text("second\n", encoding="utf-8")
    git(seed, "commit", "-q", "-am", "feat: second")
    git(seed, "push", "-q", "origin", "HEAD:main")
    return remote, seed, clone


def head(repo):
    return git(repo, "rev-parse", "HEAD").strip()


def test_a_clean_checkout_behind_its_upstream_fast_forwards(tmp_path):
    _, seed, clone = build_repo(tmp_path)
    behind = head(clone)

    result = run(clone)

    assert result.returncode == 0
    assert head(clone) == head(seed) != behind
    assert "fast-forwarded" in result.stdout


def test_an_up_to_date_checkout_is_left_alone_and_says_so(tmp_path):
    _, seed, clone = build_repo(tmp_path)
    git(clone, "fetch", "-q", "origin")
    git(clone, "merge", "-q", "--ff-only", "origin/main")
    current = head(clone)

    result = run(clone)

    assert result.returncode == 0
    assert head(clone) == current
    assert "is at origin/main" in result.stdout


def test_uncommitted_changes_block_the_fast_forward(tmp_path):
    _, seed, clone = build_repo(tmp_path)
    behind = head(clone)
    (clone / "file.txt").write_text("local edit\n", encoding="utf-8")

    result = run(clone)

    assert result.returncode != 0
    assert head(clone) == behind
    assert (clone / "file.txt").read_text(encoding="utf-8") == "local edit\n"
    assert "git stash" in result.stderr


def test_local_commits_are_never_discarded(tmp_path):
    _, _, clone = build_repo(tmp_path)
    (clone / "local.txt").write_text("mine\n", encoding="utf-8")
    git(clone, "add", "local.txt")
    git(clone, "commit", "-q", "-m", "feat: local work")
    mine = head(clone)

    result = run(clone)

    assert result.returncode == 0
    assert head(clone) == mine
    assert "commits that origin/main does not" in result.stderr


def test_a_branch_without_an_upstream_is_compared_against_nothing(tmp_path):
    git(tmp_path, "init", "-q", str(tmp_path / "solo"))
    solo = tmp_path / "solo"
    git(solo, "config", "user.email", "test@example.com")
    git(solo, "config", "user.name", "Test")
    (solo / "file.txt").write_text("only\n", encoding="utf-8")
    git(solo, "add", "file.txt")
    git(solo, "commit", "-q", "-m", "feat: only")

    result = run(solo, "--no-fetch")

    assert result.returncode == 0
    assert "no upstream" in result.stdout


def test_a_detached_head_is_reported_instead_of_moved(tmp_path):
    _, seed, clone = build_repo(tmp_path)
    git(clone, "checkout", "-q", "--detach", "HEAD")
    detached = head(clone)

    result = run(clone)

    assert result.returncode == 0
    assert head(clone) == detached
    assert "detached" in result.stderr


def test_an_unknown_option_is_rejected(tmp_path):
    result = subprocess.run(
        ["bash", str(BOOTSTRAP), "--nonsense"],
        cwd=tmp_path,
        text=True,
        capture_output=True,
        check=False,
    )

    assert result.returncode == 2
    assert "unknown option" in result.stderr


def test_running_outside_a_checkout_fails_before_touching_anything(tmp_path):
    outside = tmp_path / "plain"
    outside.mkdir()

    result = subprocess.run(
        ["bash", str(BOOTSTRAP), *QUIET],
        cwd=outside,
        text=True,
        capture_output=True,
        check=False,
        env={**os.environ, "GIT_CEILING_DIRECTORIES": str(tmp_path)},
    )

    assert result.returncode == 1
    assert "inside a Git checkout" in result.stderr
    assert not (outside / ".pixi").exists()


def test_a_branch_nobody_pushed_yet_is_not_an_error(tmp_path):
    _, _, clone = build_repo(tmp_path)
    git(clone, "switch", "-q", "-c", "session-branch")
    current = head(clone)

    result = run(clone)

    assert result.returncode == 0
    assert head(clone) == current
    assert "not on origin yet" in result.stdout
    assert "fatal" not in result.stderr
