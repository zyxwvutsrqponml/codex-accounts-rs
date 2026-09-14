#compdef codex-accounts
# zsh completion for codex-accounts
# Prints via: codex-accounts completion zsh
# Install:
#   mkdir -p ~/.zfunc
#   codex-accounts completion zsh > ~/.zfunc/_codex-accounts
#   # add to ~/.zshrc (before compinit): fpath=(~/.zfunc $fpath)
#   # then: autoload -Uz compinit && compinit

__codex_accounts_home() {
    local home="${CODEX_HOME:-$HOME/.codex}"
    case "$home" in
        "~") home="$HOME" ;;
        "~/"*) home="$HOME/${home#~/}" ;;
    esac
    print -r -- "$home"
}

__codex_accounts_profiles() {
    local home profiles_dir d
    home="$(__codex_accounts_home)"
    profiles_dir="$home/account-profiles"
    [[ -d "$profiles_dir" ]] || return 0
    for d in "$profiles_dir"/*(N/); do
        [[ -f "$d/auth.json" ]] || continue
        print -r -- "${d:t}"
    done
}

_codex-accounts() {
    local context state line
    typeset -A opt_args

    local -a commands shells
    commands=(
        'save:Save current login as a named profile'
        'use:Activate a saved profile'
        'list:List saved profiles'
        'remove:Remove a saved profile'
        'delete:Alias of remove'
        'rm:Alias of remove'
        'path:Show resolved paths'
        'new:Clear active login without server logout'
        'restart:Alias of new'
        'clear:Alias of new'
        'logout-local:Alias of new'
        'completion:Print shell completion script'
        'help:Show help'
    )
    shells=(
        'bash:Bourne again shell'
        'zsh:Z shell'
        'fish:Friendly interactive shell'
        'powershell:PowerShell'
    )

    _arguments -C \
        '--codex-home=[Override CODEX_HOME]:dir:_files -/' \
        '--no-color[Disable colors]' \
        '--plain[Plain output for scripts]' \
        '(-h --help)'{-h,--help}'[Show help]' \
        '1: :->command' \
        '*:: :->args' && return 0

    case "$state" in
        command)
            _describe -t commands 'codex-accounts command' commands
            ;;
        args)
            case "${line[1]}" in
                completion|completions|complete)
                    _describe -t shells 'shell' shells
                    ;;
                use|remove|delete|rm)
                    local -a profiles
                    profiles=(${(f)"$(__codex_accounts_profiles)"})
                    if (( ${#profiles} )); then
                        _describe -t profiles 'profile' profiles
                    fi
                    ;;
                list)
                    _arguments \
                        '--plain[Plain script output]' \
                        '--codex-home=[Override CODEX_HOME]:dir:_files -/' \
                        '--no-color[Disable colors]' \
                        '(-h --help)'{-h,--help}'[Show help]' && return 0
                    ;;
                save)
                    _arguments \
                        '--from=[Read auth from file instead of active auth.json]:file:_files' \
                        '--force[Overwrite existing profile]' \
                        '--codex-home=[Override CODEX_HOME]:dir:_files -/' \
                        '--no-color[Disable colors]' \
                        '--plain[Plain output for scripts]' \
                        '(-h --help)'{-h,--help}'[Show help]' \
                        '1:profile:->profile' && return 0
                    ;;
            esac
            ;;
    esac
}

_codex-accounts "$@"
