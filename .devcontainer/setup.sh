#!/usr/bin/env bash
# Online devcontainer bootstrap, modeled on pixi-sandbox's setup.sh.
set -euo pipefail

check_platform() {
  # The project lock and compiler aliases target linux-64, not Linux ARM64.
  if [[ "$(uname -s)" != Linux || "$(uname -m)" != x86_64 ]]; then
    echo "Clifacet's devcontainer requires Linux x86-64 (Pixi linux-64)." >&2
    exit 1
  fi
}

validate_options() {
  case "${CLIFACET_INSTALL_OPENCODE:-1}" in
    0|1) ;;
    *) echo 'CLIFACET_INSTALL_OPENCODE must be 0 or 1.' >&2; exit 2 ;;
  esac
}

check_pixi() {
  if ! command -v pixi >/dev/null 2>&1; then
    echo 'Pixi is missing: use the configured official Pixi container image.' >&2
    exit 1
  fi
  pixi --version
}

install_global_tools() {
  # The upstream Pixi image has no Git. Global tools are not in pixi.lock.
  pixi global install --channel conda-forge git gh
}

install_c_compiler() {
  # Expose the linker driver and archiver used by Rust builds.
  pixi global install --channel conda-forge \
    --expose cc --expose gcc --expose ar=x86_64-conda-linux-gnu-ar \
    c-compiler
}

install_project_environment() {
  # Fail on lock drift. The environment contains Rust, Bun and pixi-sandbox.
  pixi install --locked --all
}

install_project_clis() {
  # Install Backlog.md and skills from the frozen Bun lock.
  pixi run --frozen bun-install
}

install_opencode() {
  # Keep this optional developer tool separate from project dependencies.
  local opencode_bin
  pixi run --frozen -x bun add --global --exact opencode-ai@1.19.5
  opencode_bin="$(pixi run --frozen -x bun pm bin -g)"
  "${opencode_bin}/opencode" --version
  echo "OpenCode installed in ${opencode_bin}; model refresh and login are manual."
}

main() {
  cd "$(dirname "${BASH_SOURCE[0]}")/.."
  check_platform
  validate_options
  check_pixi

  export PATH="${HOME}/.pixi/bin:${PATH}"
  install_global_tools
  install_c_compiler
  install_project_environment
  install_project_clis

  # Opt out at container creation with CLIFACET_INSTALL_OPENCODE=0.
  if [[ "${CLIFACET_INSTALL_OPENCODE:-1}" == 1 ]]; then
    install_opencode
  fi

  echo 'Clifacet devcontainer ready. Run: pixi run --frozen quality'
}

main "$@"
