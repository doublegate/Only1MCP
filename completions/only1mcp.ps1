# PowerShell completion for only1mcp
# Installation: Add to your PowerShell profile ($PROFILE)
# Example: . path\to\only1mcp.ps1

# Register argument completer
Register-ArgumentCompleter -Native -CommandName only1mcp -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)
    
    $commands = @(
        [PSCustomObject]@{ Name = 'start'; Description = 'Start the proxy server' }
        [PSCustomObject]@{ Name = 'stop'; Description = 'Stop the running proxy server' }
        [PSCustomObject]@{ Name = 'restart'; Description = 'Restart the proxy server' }
        [PSCustomObject]@{ Name = 'status'; Description = 'Show server status' }
        [PSCustomObject]@{ Name = 'validate'; Description = 'Validate configuration file' }
        [PSCustomObject]@{ Name = 'config'; Description = 'Configuration management' }
        [PSCustomObject]@{ Name = 'help'; Description = 'Show help information' }
    )

    $configCommands = @(
        [PSCustomObject]@{ Name = 'generate'; Description = 'Generate configuration from template' }
        [PSCustomObject]@{ Name = 'validate'; Description = 'Validate configuration file' }
        [PSCustomObject]@{ Name = 'show'; Description = 'Display current configuration' }
        [PSCustomObject]@{ Name = 'edit'; Description = 'Edit configuration file' }
    )

    $templates = @(
        [PSCustomObject]@{ Name = 'solo'; Description = 'Solo developer configuration' }
        [PSCustomObject]@{ Name = 'team'; Description = 'Small team configuration' }
        [PSCustomObject]@{ Name = 'enterprise'; Description = 'Enterprise configuration' }
        [PSCustomObject]@{ Name = 'custom'; Description = 'Custom configuration' }
    )

    $logLevels = @('trace', 'debug', 'info', 'warn', 'error')
    $hosts = @('127.0.0.1', '0.0.0.0', 'localhost')
    $ports = @('8080', '3000', '4000', '5000')
    $workers = @('1', '2', '4', '8', '16')

    # Get the command parts
    $commandParts = $commandAst.ToString() -split '\s+'
    $commandCount = $commandParts.Count

    # Handle completions based on position
    if ($commandCount -le 2) {
        # Complete main commands
        $commands | Where-Object { $_.Name -like "$wordToComplete*" } | ForEach-Object {
            [System.Management.Automation.CompletionResult]::new(
                $_.Name,
                $_.Name,
                'ParameterValue',
                $_.Description
            )
        }
    }
    elseif ($commandParts[1] -eq 'start') {
        # Complete start command options
        $option = $commandParts[-2]
        
        switch ($option) {
            '--host' {
                $hosts | Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
                    [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', 'Host address')
                }
            }
            '--port' {
                $ports | Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
                    [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', 'Port number')
                }
            }
            '--log-level' {
                $logLevels | Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
                    [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', 'Log level')
                }
            }
            '--workers' {
                $workers | Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
                    [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', 'Worker threads')
                }
            }
            '--config' {
                Get-ChildItem -Path $wordToComplete* -Include *.yaml, *.yml, *.toml -ErrorAction SilentlyContinue | ForEach-Object {
                    [System.Management.Automation.CompletionResult]::new(
                        $_.FullName,
                        $_.Name,
                        'ParameterValue',
                        'Configuration file'
                    )
                }
            }
            '--log-file' {
                Get-ChildItem -Path $wordToComplete* -ErrorAction SilentlyContinue | ForEach-Object {
                    [System.Management.Automation.CompletionResult]::new(
                        $_.FullName,
                        $_.Name,
                        'ParameterValue',
                        'Log file'
                    )
                }
            }
            default {
                # Complete option flags
                $options = @(
                    [PSCustomObject]@{ Name = '--host'; Description = 'Host to bind to' }
                    [PSCustomObject]@{ Name = '--port'; Description = 'Port to bind to' }
                    [PSCustomObject]@{ Name = '--config'; Description = 'Configuration file' }
                    [PSCustomObject]@{ Name = '--daemon'; Description = 'Run as daemon' }
                    [PSCustomObject]@{ Name = '--foreground'; Description = 'Run in foreground' }
                    [PSCustomObject]@{ Name = '--workers'; Description = 'Number of worker threads' }
                    [PSCustomObject]@{ Name = '--log-level'; Description = 'Logging level' }
                    [PSCustomObject]@{ Name = '--log-file'; Description = 'Log file path' }
                    [PSCustomObject]@{ Name = '--help'; Description = 'Show help' }
                )
                
                $options | Where-Object { $_.Name -like "$wordToComplete*" } | ForEach-Object {
                    [System.Management.Automation.CompletionResult]::new(
                        $_.Name,
                        $_.Name,
                        'ParameterName',
                        $_.Description
                    )
                }
            }
        }
    }
    elseif ($commandParts[1] -eq 'config') {
        if ($commandCount -eq 3) {
            # Complete config subcommands
            $configCommands | Where-Object { $_.Name -like "$wordToComplete*" } | ForEach-Object {
                [System.Management.Automation.CompletionResult]::new(
                    $_.Name,
                    $_.Name,
                    'ParameterValue',
                    $_.Description
                )
            }
        }
        elseif ($commandParts[2] -eq 'generate') {
            $option = $commandParts[-2]
            
            if ($option -eq '--template') {
                # Complete template names
                $templates | Where-Object { $_.Name -like "$wordToComplete*" } | ForEach-Object {
                    [System.Management.Automation.CompletionResult]::new(
                        $_.Name,
                        $_.Name,
                        'ParameterValue',
                        $_.Description
                    )
                }
            }
            else {
                # Complete options
                $options = @(
                    [PSCustomObject]@{ Name = '--template'; Description = 'Template to use' }
                    [PSCustomObject]@{ Name = '--output'; Description = 'Output file' }
                )
                
                $options | Where-Object { $_.Name -like "$wordToComplete*" } | ForEach-Object {
                    [System.Management.Automation.CompletionResult]::new(
                        $_.Name,
                        $_.Name,
                        'ParameterName',
                        $_.Description
                    )
                }
            }
        }
        elseif ($commandParts[2] -in @('validate', 'show', 'edit')) {
            # Complete config file paths
            Get-ChildItem -Path $wordToComplete* -Include *.yaml, *.yml, *.toml -ErrorAction SilentlyContinue | ForEach-Object {
                [System.Management.Automation.CompletionResult]::new(
                    $_.FullName,
                    $_.Name,
                    'ParameterValue',
                    'Configuration file'
                )
            }
        }
    }
    elseif ($commandParts[1] -eq 'validate') {
        # Complete config file paths for validate command
        Get-ChildItem -Path $wordToComplete* -Include *.yaml, *.yml, *.toml -ErrorAction SilentlyContinue | ForEach-Object {
            [System.Management.Automation.CompletionResult]::new(
                $_.FullName,
                $_.Name,
                'ParameterValue',
                'Configuration file'
            )
        }
    }
    else {
        # Complete global options
        $globalOptions = @(
            [PSCustomObject]@{ Name = '--config'; Description = 'Configuration file' }
            [PSCustomObject]@{ Name = '--help'; Description = 'Show help' }
            [PSCustomObject]@{ Name = '--version'; Description = 'Show version' }
            [PSCustomObject]@{ Name = '--verbose'; Description = 'Verbose output' }
            [PSCustomObject]@{ Name = '--quiet'; Description = 'Quiet mode' }
        )
        
        $globalOptions | Where-Object { $_.Name -like "$wordToComplete*" } | ForEach-Object {
            [System.Management.Automation.CompletionResult]::new(
                $_.Name,
                $_.Name,
                'ParameterName',
                $_.Description
            )
        }
    }
}

Write-Host "Only1MCP PowerShell completion loaded. Use 'only1mcp <Tab>' for completions." -ForegroundColor Green
