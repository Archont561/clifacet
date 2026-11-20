//! Context-aware dynamic shell completions (CLI-78, decision-25).

use crate::failure::Failure;
use crate::schema::{ArgumentMetadata, CliMetadata};
use std::fmt;
use std::str::FromStr;

/// Supported shell targets for completion script generation.
#[derive(clap::ValueEnum, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Shell {
    /// Bourne-Again Shell (`Bash`).
    #[value(name = "bash")]
    Bash,
    /// Z Shell (`Zsh`).
    #[value(name = "zsh")]
    Zsh,
    /// Friendly Interactive Shell (`Fish`).
    #[value(name = "fish")]
    Fish,
    /// Microsoft `PowerShell`.
    #[value(name = "powershell", alias = "pwsh", alias = "power-shell")]
    PowerShell,
}

impl FromStr for Shell {
    type Err = Failure;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "bash" => Ok(Self::Bash),
            "zsh" => Ok(Self::Zsh),
            "fish" => Ok(Self::Fish),
            "powershell" | "pwsh" | "power-shell" => Ok(Self::PowerShell),
            other => Err(Failure::validation(format!(
                "unsupported shell '{other}': expected bash, zsh, fish, or powershell"
            ))),
        }
    }
}

impl fmt::Display for Shell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bash => write!(f, "bash"),
            Self::Zsh => write!(f, "zsh"),
            Self::Fish => write!(f, "fish"),
            Self::PowerShell => write!(f, "powershell"),
        }
    }
}

/// Generates a shell completion script for the given `shell` and command hierarchy.
#[must_use]
pub fn generate_completion_script(shell: Shell, cmd: &clap::Command) -> String {
    let meta = CliMetadata::from_command(cmd);
    generate_completion_script_from_meta(shell, &meta)
}

/// Generates a shell completion script from extracted [`CliMetadata`].
#[must_use]
pub fn generate_completion_script_from_meta(shell: Shell, meta: &CliMetadata) -> String {
    let bin = &meta.name;
    match shell {
        Shell::Bash => generate_bash(bin, meta),
        Shell::Zsh => generate_zsh(bin, meta),
        Shell::Fish => generate_fish(bin, meta),
        Shell::PowerShell => generate_powershell(bin),
    }
}

fn generate_bash(bin: &str, meta: &CliMetadata) -> String {
    let bin_ident = bin.replace('-', "_");
    let top_commands: Vec<&str> = meta.subcommands.iter().map(|s| s.name.as_str()).collect();
    let top_flags: Vec<String> = meta
        .arguments
        .iter()
        .filter_map(|a| a.long.as_ref().map(|l| format!("--{l}")))
        .collect();
    let mut static_words = top_commands;
    let flag_strs: Vec<&str> = top_flags.iter().map(String::as_str).collect();
    static_words.extend(flag_strs);
    static_words.push("--help");
    static_words.push("--version");
    let words_joined = static_words.join(" ");

    format!(
        r#"# Bash completion script for {bin}
_{bin_ident}_completions() {{
    local cur prev words cword
    if declare -F _init_completion >/dev/null 2>&1; then
        _init_completion -n : || return
    else
        COMPREPLY=()
        cur="${{COMP_WORDS[COMP_CWORD]}}"
        prev="${{COMP_WORDS[COMP_CWORD-1]}}"
        words=("${{COMP_WORDS[@]}}")
        cword=$COMP_CWORD
    fi

    local candidates
    candidates=$("{bin}" completions --complete -- "${{words[@]:1:$cword}}" 2>/dev/null)
    if [ $? -eq 0 ] && [ -n "$candidates" ]; then
        local IFS=$'\n'
        COMPREPLY=($(compgen -W "$candidates" -- "$cur"))
        return 0
    fi

    COMPREPLY=($(compgen -W "{words_joined}" -- "$cur"))
    return 0
}}

complete -F _{bin_ident}_completions -o default {bin}
"#
    )
}

fn generate_zsh(bin: &str, meta: &CliMetadata) -> String {
    let bin_ident = bin.replace('-', "_");
    let mut subcmd_specs = Vec::new();
    for sub in &meta.subcommands {
        let desc = sub.about.as_deref().unwrap_or("").replace('\'', "'\\''");
        subcmd_specs.push(format!("'{}':'{}'", sub.name, desc));
    }
    let subcmds_joined = subcmd_specs.join("\n        ");

    format!(
        r#"#compdef {bin}

_{bin_ident}() {{
    local -a response
    local -a word_args

    word_args=("${{words[@]:1:$CURRENT-1}}")
    response=($({bin} completions --complete -- "${{word_args[@]}}" 2>/dev/null))

    if [ $? -eq 0 ] && [ ${{#response[@]}} -gt 0 ]; then
        _describe -t values '{bin}' response
        return 0
    fi

    local -a subcommands
    subcommands=(
        {subcmds_joined}
    )
    _describe -t commands 'subcommands' subcommands
}}

_{bin_ident} "$@"
"#
    )
}

fn generate_fish(bin: &str, meta: &CliMetadata) -> String {
    use std::fmt::Write as _;

    let bin_ident = bin.replace('-', "_");
    let mut script = format!(
        r#"# Fish completion script for {bin}

function __fish_{bin_ident}_complete
    set -l tokens (commandline -co)
    set -l current (commandline -ct)
    set -l cmd_args $tokens[2..-1] $current
    {bin} completions --complete -- $cmd_args 2>/dev/null
end

complete -c {bin} -f -a "(__fish_{bin_ident}_complete)"
"#
    );

    for sub in &meta.subcommands {
        let desc = sub.about.as_deref().unwrap_or("").replace('\'', "\\'");
        let _ = writeln!(
            script,
            "complete -c {bin} -n \"__fish_use_subcommand\" -a \"{}\" -d '{desc}'",
            sub.name
        );
    }
    for arg in &meta.arguments {
        if let Some(long) = &arg.long {
            let desc = arg.help.as_deref().unwrap_or("").replace('\'', "\\'");
            let _ = writeln!(script, "complete -c {bin} -l \"{long}\" -d '{desc}'");
        }
    }
    script
}

fn generate_powershell(bin: &str) -> String {
    format!(
        r"# PowerShell completion script for {bin}
Register-ArgumentCompleter -Native -CommandName '{bin}' -ScriptBlock {{
    param($wordToComplete, $commandAst, $cursorPosition)
    $words = $commandAst.CommandElements | Select-Object -ExpandProperty Value
    $cmdArgs = @()
    if ($words.Length -gt 1) {{
        $cmdArgs = $words[1..($words.Length - 1)]
    }}
    if ($wordToComplete) {{
        $cmdArgs += $wordToComplete
    }}
    $results = & {bin} completions --complete -- $cmdArgs 2>$null
    if ($results) {{
        $results | ForEach-Object {{
            [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
        }}
    }}
}}
"
    )
}

/// Resolves completion candidates given the CLI metadata, word list, and dynamic argument callback.
///
/// `callback` takes `(subcommand, argument_id, prefix)` and returns matching candidates if defined.
#[allow(clippy::too_many_lines)]
pub fn resolve_completions<F>(meta: &CliMetadata, words: &[String], callback: F) -> Vec<String>
where
    F: Fn(&str, &str, &str) -> Option<Vec<String>>,
{
    if words.is_empty() {
        return collect_subcommands_and_flags(meta, "");
    }

    let current = words.last().map(String::as_str).unwrap_or_default();
    let prior = &words[..words.len() - 1];

    // Traverse prior tokens to identify the active subcommand
    let mut active = meta;
    let mut positional_count = 0;
    let mut skip_next = false;

    for (idx, token) in prior.iter().enumerate() {
        if skip_next {
            skip_next = false;
            continue;
        }

        if let Some(sub) = active.subcommands.iter().find(|s| s.name == *token) {
            active = sub;
            positional_count = 0;
            continue;
        }

        if token.starts_with("--") {
            if token.contains('=') {
                continue;
            }
            let flag = token.trim_start_matches('-');
            if let Some(arg) = active
                .arguments
                .iter()
                .find(|a| a.long.as_deref() == Some(flag))
                && !arg.is_flag
                && idx + 1 < prior.len()
            {
                skip_next = true;
            }
        } else if token.starts_with('-') {
            let short_ch = token.chars().nth(1);
            if let Some(arg) = active.arguments.iter().find(|a| a.short == short_ch)
                && !arg.is_flag
                && idx + 1 < prior.len()
            {
                skip_next = true;
            }
        } else {
            positional_count += 1;
        }
    }

    // Case 1: Flag with inline value `--flag=val`
    if let Some((flag_part, val_part)) = current.split_once('=') {
        let flag_clean = flag_part.trim_start_matches('-');
        if let Some(arg) = active
            .arguments
            .iter()
            .find(|a| a.long.as_deref() == Some(flag_clean) || a.id == flag_clean)
        {
            if let Some(dynamic) = callback(&active.name, &arg.id, val_part) {
                return dynamic
                    .into_iter()
                    .filter(|item| item.starts_with(val_part))
                    .map(|item| format!("{flag_part}={item}"))
                    .collect();
            }
            if !arg.possible_values.is_empty() {
                return arg
                    .possible_values
                    .iter()
                    .filter(|pv| pv.starts_with(val_part))
                    .map(|pv| format!("{flag_part}={pv}"))
                    .collect();
            }
        }
        return Vec::new();
    }

    // Case 2: The prior token was a flag expecting a value
    if let Some(prev) = prior.last()
        && prev.starts_with('-')
        && !prev.contains('=')
    {
        let flag_clean = prev.trim_start_matches('-');
        let matched_arg = active.arguments.iter().find(|a| {
            a.long.as_deref() == Some(flag_clean)
                || a.id == flag_clean
                || a.short.is_some_and(|s| s.to_string() == flag_clean)
        });

        if let Some(arg) = matched_arg
            && !arg.is_flag
        {
            if let Some(dynamic) = callback(&active.name, &arg.id, current) {
                return dynamic
                    .into_iter()
                    .filter(|item| item.starts_with(current))
                    .collect();
            }
            if !arg.possible_values.is_empty() {
                return arg
                    .possible_values
                    .iter()
                    .filter(|pv| pv.starts_with(current))
                    .cloned()
                    .collect();
            }
        }
    }

    // Case 3: Completing a flag name
    if current.starts_with('-') {
        let mut flags = Vec::new();
        for arg in &active.arguments {
            if let Some(long) = &arg.long {
                let f = format!("--{long}");
                if f.starts_with(current) {
                    flags.push(f);
                }
            }
            if let Some(short) = &arg.short {
                let f = format!("-{short}");
                if f.starts_with(current) {
                    flags.push(f);
                }
            }
        }
        if "--help".starts_with(current) {
            flags.push("--help".to_string());
        }
        if "--version".starts_with(current) {
            flags.push("--version".to_string());
        }
        return flags;
    }

    // Case 4: Subcommand or positional argument
    let mut results = Vec::new();

    // Matching subcommands
    for sub in &active.subcommands {
        if sub.name.starts_with(current) {
            results.push(sub.name.clone());
        }
    }

    // Matching positional argument dynamic callback or possible values
    let positionals: Vec<&ArgumentMetadata> =
        active.arguments.iter().filter(|a| a.positional).collect();
    if let Some(pos_arg) = positionals.get(positional_count) {
        if let Some(dynamic) = callback(&active.name, &pos_arg.id, current) {
            let matched = dynamic.into_iter().filter(|item| item.starts_with(current));
            results.extend(matched);
        } else if !pos_arg.possible_values.is_empty() {
            let matched = pos_arg
                .possible_values
                .iter()
                .filter(|pv| pv.starts_with(current))
                .cloned();
            results.extend(matched);
        }
    }

    results
}

fn collect_subcommands_and_flags(meta: &CliMetadata, prefix: &str) -> Vec<String> {
    let mut results = Vec::new();
    for sub in &meta.subcommands {
        if sub.name.starts_with(prefix) {
            results.push(sub.name.clone());
        }
    }
    for arg in &meta.arguments {
        if let Some(long) = &arg.long {
            let f = format!("--{long}");
            if f.starts_with(prefix) {
                results.push(f);
            }
        }
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_parsing_and_display() {
        assert_eq!("bash".parse::<Shell>().unwrap(), Shell::Bash);
        assert_eq!("zsh".parse::<Shell>().unwrap(), Shell::Zsh);
        assert_eq!("fish".parse::<Shell>().unwrap(), Shell::Fish);
        assert_eq!("powershell".parse::<Shell>().unwrap(), Shell::PowerShell);
        assert_eq!("pwsh".parse::<Shell>().unwrap(), Shell::PowerShell);
        assert_eq!("power-shell".parse::<Shell>().unwrap(), Shell::PowerShell);
        assert!("invalid".parse::<Shell>().is_err());

        assert_eq!(Shell::Bash.to_string(), "bash");
        assert_eq!(Shell::Zsh.to_string(), "zsh");
        assert_eq!(Shell::Fish.to_string(), "fish");
        assert_eq!(Shell::PowerShell.to_string(), "powershell");
    }

    #[test]
    fn static_completion_generation() {
        let cmd = clap::Command::new("demo-cli")
            .about("A demo CLI")
            .subcommand(
                clap::Command::new("status").about("Get status").arg(
                    clap::Arg::new("verbose")
                        .long("verbose")
                        .action(clap::ArgAction::SetTrue),
                ),
            )
            .subcommand(
                clap::Command::new("deploy")
                    .about("Deploy target")
                    .arg(clap::Arg::new("cluster").long("cluster").num_args(1)),
            );

        let bash = generate_completion_script(Shell::Bash, &cmd);
        assert!(bash.contains("_demo_cli_completions()"));
        assert!(bash.contains("complete -F _demo_cli_completions -o default demo-cli"));

        let zsh = generate_completion_script(Shell::Zsh, &cmd);
        assert!(zsh.contains("#compdef demo-cli"));
        assert!(zsh.contains("_demo_cli()"));

        let fish = generate_completion_script(Shell::Fish, &cmd);
        assert!(fish.contains("complete -c demo-cli"));
        assert!(fish.contains("__fish_demo_cli_complete"));

        let pwsh = generate_completion_script(Shell::PowerShell, &cmd);
        assert!(pwsh.contains("Register-ArgumentCompleter"));
        assert!(pwsh.contains("demo-cli"));
    }

    #[test]
    fn dynamic_completion_resolution() {
        let cmd = clap::Command::new("test-cli")
            .subcommand(
                clap::Command::new("deploy")
                    .arg(clap::Arg::new("cluster").long("cluster").num_args(1))
                    .arg(clap::Arg::new("image").index(1).num_args(1)),
            )
            .subcommand(clap::Command::new("status"));
        let meta = CliMetadata::from_command(&cmd);

        // 1. Top-level completion
        let words = vec!["dep".to_string()];
        let resolved = resolve_completions(&meta, &words, |_cmd, _arg, _cur| None);
        assert_eq!(resolved, vec!["deploy"]);

        // 2. Flags completion
        let words = vec!["deploy".to_string(), "--c".to_string()];
        let resolved = resolve_completions(&meta, &words, |_cmd, _arg, _cur| None);
        assert_eq!(resolved, vec!["--cluster"]);

        // 3. Dynamic callback for flag value
        let words = vec![
            "deploy".to_string(),
            "--cluster".to_string(),
            "st".to_string(),
        ];
        let resolved = resolve_completions(&meta, &words, |subcmd, arg, _cur| {
            if subcmd == "deploy" && arg == "cluster" {
                Some(vec![
                    "staging-1".to_string(),
                    "staging-2".to_string(),
                    "prod-1".to_string(),
                ])
            } else {
                None
            }
        });
        assert_eq!(resolved, vec!["staging-1", "staging-2"]);

        // 4. Dynamic callback with `--cluster=st`
        let words = vec!["deploy".to_string(), "--cluster=st".to_string()];
        let resolved = resolve_completions(&meta, &words, |subcmd, arg, _cur| {
            if subcmd == "deploy" && arg == "cluster" {
                Some(vec!["staging-1".to_string(), "prod-1".to_string()])
            } else {
                None
            }
        });
        assert_eq!(resolved, vec!["--cluster=staging-1"]);

        // 5. Positional argument dynamic callback
        let words = vec!["deploy".to_string(), "ng".to_string()];
        let resolved = resolve_completions(&meta, &words, |subcmd, arg, _cur| {
            if subcmd == "deploy" && arg == "image" {
                Some(vec!["nginx:alpine".to_string(), "redis:7".to_string()])
            } else {
                None
            }
        });
        assert_eq!(resolved, vec!["nginx:alpine"]);
    }
}
