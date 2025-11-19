# Shell Completions for Only1MCP

Auto-completion scripts for various shells to enhance the command-line experience.

## Installation

### Bash

```bash
# System-wide installation (requires root)
sudo cp only1mcp.bash /etc/bash_completion.d/

# User installation
mkdir -p ~/.local/share/bash-completion/completions
cp only1mcp.bash ~/.local/share/bash-completion/completions/only1mcp

# Manual sourcing (temporary)
source only1mcp.bash
```

Restart your shell or run `source ~/.bashrc` to activate.

### Zsh

```bash
# Find your fpath directories
echo $fpath

# Copy to one of the fpath directories (choose one)
sudo cp _only1mcp /usr/local/share/zsh/site-functions/
# OR
cp _only1mcp ~/.zsh/completions/  # (add to fpath if needed)

# If using custom directory, add to your ~/.zshrc:
fpath=(~/.zsh/completions $fpath)
autoload -Uz compinit && compinit
```

Restart your shell or run `exec zsh` to activate.

### Fish

```bash
# System-wide installation
sudo cp only1mcp.fish /usr/share/fish/vendor_completions.d/

# User installation (recommended)
mkdir -p ~/.config/fish/completions
cp only1mcp.fish ~/.config/fish/completions/
```

Fish will automatically load completions on next shell start.

### PowerShell

```powershell
# Find your profile path
$PROFILE

# If profile doesn't exist, create it
if (!(Test-Path -Path $PROFILE)) {
    New-Item -ItemType File -Path $PROFILE -Force
}

# Add to your profile
Add-Content -Path $PROFILE -Value ". path\to\only1mcp.ps1"

# Or manually add this line to your profile:
. C:\path\to\only1mcp\completions\only1mcp.ps1

# Reload profile
. $PROFILE
```

## Usage Examples

Once installed, tab completion works for all commands:

### Command Completion

```bash
only1mcp <TAB>
# Shows: start stop restart status validate config help

only1mcp st<TAB>
# Completes to: only1mcp start
```

### Subcommand Completion

```bash
only1mcp config <TAB>
# Shows: generate validate show edit

only1mcp config gen<TAB>
# Completes to: only1mcp config generate
```

### Option Completion

```bash
only1mcp start --<TAB>
# Shows: --host --port --config --daemon --foreground --workers --log-level --log-file --help

only1mcp start --log-level <TAB>
# Shows: trace debug info warn error
```

### File Path Completion

```bash
only1mcp --config <TAB>
# Shows available .yaml, .yml, and .toml files

only1mcp config generate --template <TAB>
# Shows: solo team enterprise custom
```

## Supported Completions

### Commands
- `start` - Start the proxy server
- `stop` - Stop the running server
- `restart` - Restart the server
- `status` - Show server status
- `validate` - Validate configuration
- `config` - Configuration management
- `help` - Show help

### Config Subcommands
- `generate` - Generate configuration from template
- `validate` - Validate configuration file
- `show` - Display current configuration
- `edit` - Edit configuration file

### Templates
- `solo` - Solo developer configuration
- `team` - Small team configuration
- `enterprise` - Enterprise configuration
- `custom` - Custom configuration

### Common Options
- `--config` - Configuration file path
- `--host` - Host to bind to
- `--port` - Port to bind to
- `--log-level` - Logging level (trace/debug/info/warn/error)
- `--workers` - Number of worker threads
- `--daemon` - Run as daemon
- `--help` - Show help
- `--version` - Show version

## Testing

After installation, test that completions work:

```bash
# Type this and press TAB (don't press Enter)
only1mcp <TAB><TAB>

# You should see a list of available commands
```

## Troubleshooting

### Bash

If completions don't work:

1. Check if bash-completion is installed:
   ```bash
   dpkg -l | grep bash-completion  # Debian/Ubuntu
   rpm -qa | grep bash-completion  # RHEL/Fedora
   ```

2. Ensure bash-completion is sourced in your `.bashrc`:
   ```bash
   if [ -f /etc/bash_completion ]; then
       . /etc/bash_completion
   fi
   ```

3. Try sourcing directly:
   ```bash
   source /path/to/only1mcp.bash
   ```

### Zsh

If completions don't work:

1. Check if compinit is enabled in `.zshrc`:
   ```bash
   autoload -Uz compinit && compinit
   ```

2. Check fpath:
   ```bash
   echo $fpath
   ```

3. Rebuild completion cache:
   ```bash
   rm -f ~/.zcompdump
   compinit
   ```

### Fish

If completions don't work:

1. Check fish version (3.0+  required):
   ```bash
   fish --version
   ```

2. List loaded completions:
   ```bash
   complete -c only1mcp
   ```

3. Reload completions:
   ```bash
   fish_update_completions
   ```

### PowerShell

If completions don't work:

1. Check execution policy:
   ```powershell
   Get-ExecutionPolicy
   # Should be RemoteSigned or Unrestricted
   Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
   ```

2. Verify profile is loading:
   ```powershell
   Test-Path $PROFILE
   Get-Content $PROFILE
   ```

3. Manually source the script:
   ```powershell
   . .\only1mcp.ps1
   ```

## Contributing

To add completions for additional shells or improve existing ones, please submit a pull request.

## License

These completion scripts are part of Only1MCP and are licensed under GPL-3.0.
