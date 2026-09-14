# PowerShell completion for codex-accounts
# Prints via: codex-accounts completion powershell
# Install (add to $PROFILE):
#   codex-accounts completion powershell | Out-String | Invoke-Expression
# Or save to a file and dot-source it from $PROFILE.

Register-ArgumentCompleter -Native -CommandName @('codex-accounts') -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commands = @('save', 'use', 'list', 'remove', 'delete', 'rm', 'path', 'new', 'restart', 'clear', 'logout-local', 'completion', 'help')
    $shells = @('bash', 'zsh', 'fish', 'powershell')
    $globalFlags = @('--codex-home', '--no-color', '--plain', '--help', '-h')

    function Get-CodexHome {
        $homeDir = $env:CODEX_HOME
        if ([string]::IsNullOrEmpty($homeDir)) {
            $homeDir = Join-Path $HOME '.codex'
        }
        return $homeDir
    }

    function Get-CodexProfiles {
        $profilesDir = Join-Path (Get-CodexHome) 'account-profiles'
        if (-not (Test-Path $profilesDir)) { return @() }
        Get-ChildItem -Directory $profilesDir -ErrorAction SilentlyContinue |
            Where-Object { Test-Path (Join-Path $_.FullName 'auth.json') } |
            Select-Object -ExpandProperty Name
    }

    $elements = $commandAst.CommandElements
    # Find subcommand: first non-flag token after the binary name.
    $command = $null
    $skipNext = $false
    for ($i = 1; $i -lt $elements.Count; $i++) {
        $text = "$($elements[$i])"
        if ($skipNext) { $skipNext = $false; continue }
        if ($text -eq '--codex-home' -or $text -eq '--from') { $skipNext = $true; continue }
        if ($text.StartsWith('--codex-home=') -or $text.StartsWith('--from=')) { continue }
        if ($text.StartsWith('-')) { continue }
        $command = $text
        break
    }

    if ([string]::IsNullOrEmpty($command)) {
        if ($wordToComplete.StartsWith('-')) {
            $globalFlags | Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
                [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
            }
            return
        }
        $commands | Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
            [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
        }
        return
    }

    switch ($command) {
        { $_ -in 'completion', 'completions', 'complete' } {
            $shells | Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
                [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
            }
        }
        { $_ -in 'use', 'remove', 'delete', 'rm', 'save' } {
            Get-CodexProfiles | Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
                [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
            }
        }
    }
}
