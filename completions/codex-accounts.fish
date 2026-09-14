# fish completion for codex-accounts
# Prints via: codex-accounts completion fish
# Install:
#   mkdir -p ~/.config/fish/completions
#   codex-accounts completion fish > ~/.config/fish/completions/codex-accounts.fish

function __codex_accounts_home
    if set -q CODEX_HOME; and test -n "$CODEX_HOME"
        echo "$CODEX_HOME"
    else
        echo "$HOME/.codex"
    end
end

function __codex_accounts_profiles
    set -l profiles_dir (__codex_accounts_home)/account-profiles
    test -d "$profiles_dir"; or return 0
    for d in "$profiles_dir"/*/
        test -f "$d/auth.json"; or continue
        basename "$d"
    end
end

function __codex_accounts_needs_command
    set -l cmd (commandline -opc)
    # skip `codex-accounts` itself and any global flags/values
    set -e cmd[1]
    for i in $cmd
        switch "$i"
            case '--codex-home'
                # value follows; both consumed below via index handling is
                # overkill here — if any non-flag token remains, we have a command
                continue
            case '--codex-home=*'
                continue
            case '-*'
                continue
            case '*'
                return 1
        end
    end
    return 0
end

function __codex_accounts_using_command
    set -l cmd (commandline -opc)
    set -e cmd[1]
    set -l skip 0
    for i in $cmd
        if test "$skip" = 1
            set skip 0
            continue
        end
        switch "$i"
            case '--codex-home' '--from'
                set skip 1
            case '--codex-home=*' '--from=*'
            case '-*'
            case '*'
                contains -- "$i" $argv
                and return 0
                return 1
        end
    end
    return 1
end

# Global flags
complete -c codex-accounts -f -s h -l help --description 'Show help'
complete -c codex-accounts -f -l codex-home -r --description 'Override CODEX_HOME'
complete -c codex-accounts -f -l no-color --description 'Disable colors'
complete -c codex-accounts -f -l plain --description 'Plain output for scripts'

# Subcommands
complete -c codex-accounts -f -n '__codex_accounts_needs_command' -a save --description 'Save current login as profile'
complete -c codex-accounts -f -n '__codex_accounts_needs_command' -a use --description 'Activate a saved profile'
complete -c codex-accounts -f -n '__codex_accounts_needs_command' -a list --description 'List saved profiles'
complete -c codex-accounts -f -n '__codex_accounts_needs_command' -a remove --description 'Remove a saved profile'
complete -c codex-accounts -f -n '__codex_accounts_needs_command' -a delete --description 'Alias of remove'
complete -c codex-accounts -f -n '__codex_accounts_needs_command' -a rm --description 'Alias of remove'
complete -c codex-accounts -f -n '__codex_accounts_needs_command' -a path --description 'Show resolved paths'
complete -c codex-accounts -f -n '__codex_accounts_needs_command' -a new --description 'Clear active login without server logout'
complete -c codex-accounts -f -n '__codex_accounts_needs_command' -a restart --description 'Alias of new'
complete -c codex-accounts -f -n '__codex_accounts_needs_command' -a clear --description 'Alias of new'
complete -c codex-accounts -f -n '__codex_accounts_needs_command' -a logout-local --description 'Alias of new'
complete -c codex-accounts -f -n '__codex_accounts_needs_command' -a completion --description 'Print shell completion script'
complete -c codex-accounts -f -n '__codex_accounts_needs_command' -a help --description 'Show help'

# Profile names (typing `a<TAB>` completes `abcd`)
complete -c codex-accounts -f -n '__codex_accounts_using_command use' -a '(__codex_accounts_profiles)' --description 'Profile'
complete -c codex-accounts -f -n '__codex_accounts_using_command remove' -a '(__codex_accounts_profiles)' --description 'Profile'
complete -c codex-accounts -f -n '__codex_accounts_using_command delete' -a '(__codex_accounts_profiles)' --description 'Profile'
complete -c codex-accounts -f -n '__codex_accounts_using_command rm' -a '(__codex_accounts_profiles)' --description 'Profile'
complete -c codex-accounts -f -n '__codex_accounts_using_command save' -a '(__codex_accounts_profiles)' --description 'Profile'
complete -c codex-accounts -f -n '__codex_accounts_using_command save' -l from -r --description 'Read auth from file'
complete -c codex-accounts -f -n '__codex_accounts_using_command save' -l force --description 'Overwrite existing profile'
complete -c codex-accounts -f -n '__codex_accounts_using_command list' -l plain --description 'Plain script output'

# Shell names
complete -c codex-accounts -f -n '__codex_accounts_using_command completion' -a 'bash zsh fish powershell'
complete -c codex-accounts -f -n '__codex_accounts_using_command completions' -a 'bash zsh fish powershell'
complete -c codex-accounts -f -n '__codex_accounts_using_command complete' -a 'bash zsh fish powershell'
