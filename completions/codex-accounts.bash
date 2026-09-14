# bash completion for codex-accounts
# Prints via: codex-accounts completion bash
# Install (pick one):
#   mkdir -p ~/.local/share/bash-completion/completions
#   codex-accounts completion bash > ~/.local/share/bash-completion/completions/codex-accounts
#   # then restart your shell (bash-completion must be enabled)
# Manual: source <(codex-accounts completion bash)

__codex_accounts_home() {
    local home="" i w
    for (( i=1; i<COMP_CWORD; i++ )); do
        w="${COMP_WORDS[i]}"
        case "$w" in
            --codex-home)
                home="${COMP_WORDS[i+1]:-}"
                break
                ;;
            --codex-home=*)
                home="${w#--codex-home=}"
                break
                ;;
        esac
    done
    if [ -z "$home" ]; then
        home="${CODEX_HOME:-$HOME/.codex}"
    fi
    case "$home" in
        "~") home="$HOME" ;;
        "~/"*) home="$HOME/${home#~/}" ;;
    esac
    printf '%s' "$home"
}

__codex_accounts_profiles() {
    local home profiles_dir d name
    home="$(__codex_accounts_home)"
    profiles_dir="$home/account-profiles"
    [ -d "$profiles_dir" ] || return 0
    for d in "$profiles_dir"/*/; do
        [ -d "$d" ] || continue
        [ -f "${d}auth.json" ] || continue
        name="$(basename "$d")"
        printf '%s\n' "$name"
    done
}

_codex_accounts() {
    local cur prev command i w skip
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev=""
    if [ "$COMP_CWORD" -ge 1 ]; then
        prev="${COMP_WORDS[COMP_CWORD-1]}"
    fi

    # --codex-home / --from take a filesystem path.
    case "$prev" in
        --codex-home|--from)
            COMPREPLY=( $(compgen -f -- "$cur") )
            return 0
            ;;
    esac
    case "$cur" in
        --codex-home=*|--from=*)
            local prefix="${cur%%=*}=" stem="${cur#*=}"
            COMPREPLY=( $(compgen -f -- "$stem" | while IFS= read -r f; do printf '%s\n' "$prefix$f"; done) )
            return 0
            ;;
    esac

    # Find the subcommand: first non-flag word (skipping --codex-home value).
    command=""
    skip=0
    for (( i=1; i<COMP_CWORD; i++ )); do
        w="${COMP_WORDS[i]}"
        if [ "$skip" -eq 1 ]; then
            skip=0
            continue
        fi
        case "$w" in
            --codex-home|--from) skip=1 ;;
            --codex-home=*|--from=*) ;;
            --*) ;;
            -*) ;;
            *) command="$w"; break ;;
        esac
    done

    # No subcommand yet: complete commands + global flags.
    if [ -z "$command" ]; then
        if [[ "$cur" == -* ]]; then
            COMPREPLY=( $(compgen -W "--codex-home --no-color --plain --help" -- "$cur") )
        else
            COMPREPLY=( $(compgen -W "save use list remove delete rm path new restart clear logout-local completion help" -- "$cur") )
        fi
        return 0
    fi

    case "$command" in
        completion|completions|complete)
            COMPREPLY=( $(compgen -W "bash zsh fish powershell" -- "$cur") )
            return 0
            ;;
        use|remove|delete|rm)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "--codex-home --no-color --plain --help" -- "$cur") )
            else
                COMPREPLY=( $(compgen -W "$(__codex_accounts_profiles)" -- "$cur") )
            fi
            return 0
            ;;
        save)
            case "$prev" in
                --from) COMPREPLY=( $(compgen -f -- "$cur") ); return 0 ;;
            esac
            if [[ "$cur" == --from=* ]]; then
                return 0
            fi
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "--from --force --codex-home --no-color --plain --help" -- "$cur") )
            else
                COMPREPLY=( $(compgen -W "$(__codex_accounts_profiles)" -- "$cur") )
            fi
            return 0
            ;;
        list)
            COMPREPLY=( $(compgen -W "--plain --codex-home --no-color --help" -- "$cur") )
            return 0
            ;;
        new|restart|clear|logout-local|path|help)
            COMPREPLY=( $(compgen -W "--codex-home --no-color --plain --help" -- "$cur") )
            return 0
            ;;
        *)
            return 0
            ;;
    esac
}

complete -F _codex_accounts codex-accounts
