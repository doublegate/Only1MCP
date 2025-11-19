# Fish completion for only1mcp
# Installation: Copy to ~/.config/fish/completions/ or /usr/share/fish/vendor_completions.d/

# Remove any existing completions
complete -c only1mcp -e

# Main commands
complete -c only1mcp -f -n "__fish_use_subcommand" -a start -d "Start the proxy server"
complete -c only1mcp -f -n "__fish_use_subcommand" -a stop -d "Stop the running proxy server"
complete -c only1mcp -f -n "__fish_use_subcommand" -a restart -d "Restart the proxy server"
complete -c only1mcp -f -n "__fish_use_subcommand" -a status -d "Show server status"
complete -c only1mcp -f -n "__fish_use_subcommand" -a validate -d "Validate configuration file"
complete -c only1mcp -f -n "__fish_use_subcommand" -a config -d "Configuration management"
complete -c only1mcp -f -n "__fish_use_subcommand" -a help -d "Show help information"

# Global options
complete -c only1mcp -l config -d "Configuration file" -r -F -a "*.yaml *.yml *.toml"
complete -c only1mcp -l help -d "Show help information"
complete -c only1mcp -l version -d "Show version"
complete -c only1mcp -l verbose -d "Verbose output"
complete -c only1mcp -l quiet -d "Quiet mode"

# Start command options
complete -c only1mcp -n "__fish_seen_subcommand_from start" -l host -d "Host to bind to" -x -a "127.0.0.1 0.0.0.0 localhost"
complete -c only1mcp -n "__fish_seen_subcommand_from start" -l port -d "Port to bind to" -x -a "8080 3000 4000 5000"
complete -c only1mcp -n "__fish_seen_subcommand_from start" -l daemon -d "Run as daemon"
complete -c only1mcp -n "__fish_seen_subcommand_from start" -l foreground -d "Run in foreground"
complete -c only1mcp -n "__fish_seen_subcommand_from start" -l workers -d "Number of worker threads" -x -a "1 2 4 8 16"
complete -c only1mcp -n "__fish_seen_subcommand_from start" -l log-level -d "Logging level" -x -a "trace debug info warn error"
complete -c only1mcp -n "__fish_seen_subcommand_from start" -l log-file -d "Log file path" -r -F

# Config subcommands
complete -c only1mcp -n "__fish_seen_subcommand_from config; and not __fish_seen_subcommand_from generate validate show edit" -f -a generate -d "Generate configuration from template"
complete -c only1mcp -n "__fish_seen_subcommand_from config; and not __fish_seen_subcommand_from generate validate show edit" -f -a validate -d "Validate configuration file"
complete -c only1mcp -n "__fish_seen_subcommand_from config; and not __fish_seen_subcommand_from generate validate show edit" -f -a show -d "Display current configuration"
complete -c only1mcp -n "__fish_seen_subcommand_from config; and not __fish_seen_subcommand_from generate validate show edit" -f -a edit -d "Edit configuration file"

# Config generate options
complete -c only1mcp -n "__fish_seen_subcommand_from config; and __fish_seen_subcommand_from generate" -l template -d "Template to use" -x -a "solo team enterprise custom"
complete -c only1mcp -n "__fish_seen_subcommand_from config; and __fish_seen_subcommand_from generate" -l output -d "Output file" -r -F

# Config validate/show/edit options
complete -c only1mcp -n "__fish_seen_subcommand_from config; and __fish_seen_subcommand_from validate show edit" -r -F -a "*.yaml *.yml *.toml"

# Validate command options
complete -c only1mcp -n "__fish_seen_subcommand_from validate" -r -F -a "*.yaml *.yml *.toml"
