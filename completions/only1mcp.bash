# Bash completion for only1mcp
# Installation: source this file or copy to /etc/bash_completion.d/

_only1mcp() {
    local cur prev opts base
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"

    # Main commands
    local commands="start stop restart status validate config help"

    # Global options
    local global_opts="--config --help --version --verbose --quiet"

    # Start command options
    local start_opts="--host --port --daemon --foreground --workers --log-level --log-file"

    # Config command subcommands
    local config_cmds="generate validate show edit"

    # Config templates
    local templates="solo team enterprise custom"

    # Handle completions based on position
    case "${COMP_CWORD}" in
        1)
            # Complete main commands
            COMPREPLY=( $(compgen -W "${commands}" -- ${cur}) )
            return 0
            ;;
        2)
            case "${prev}" in
                config)
                    # Complete config subcommands
                    COMPREPLY=( $(compgen -W "${config_cmds}" -- ${cur}) )
                    return 0
                    ;;
                start)
                    # Complete start options
                    COMPREPLY=( $(compgen -W "${start_opts}" -- ${cur}) )
                    return 0
                    ;;
                *)
                    # Complete global options
                    COMPREPLY=( $(compgen -W "${global_opts}" -- ${cur}) )
                    return 0
                    ;;
            esac
            ;;
        3)
            case "${COMP_WORDS[1]}" in
                config)
                    case "${prev}" in
                        generate)
                            # Complete template options
                            COMPREPLY=( $(compgen -W "--template ${templates}" -- ${cur}) )
                            return 0
                            ;;
                        validate|show|edit)
                            # Complete file paths
                            COMPREPLY=( $(compgen -f -- ${cur}) )
                            return 0
                            ;;
                    esac
                    ;;
            esac
            ;;
    esac

    # Handle option values
    case "${prev}" in
        --config)
            # Complete config file paths
            COMPREPLY=( $(compgen -f -X '!*.@(yaml|yml|toml)' -- ${cur}) )
            return 0
            ;;
        --template)
            # Complete template names
            COMPREPLY=( $(compgen -W "${templates}" -- ${cur}) )
            return 0
            ;;
        --host)
            # Suggest common host values
            COMPREPLY=( $(compgen -W "127.0.0.1 0.0.0.0 localhost" -- ${cur}) )
            return 0
            ;;
        --port)
            # Suggest common ports
            COMPREPLY=( $(compgen -W "8080 3000 4000 5000" -- ${cur}) )
            return 0
            ;;
        --log-level)
            # Complete log levels
            COMPREPLY=( $(compgen -W "trace debug info warn error" -- ${cur}) )
            return 0
            ;;
        --log-file)
            # Complete file paths
            COMPREPLY=( $(compgen -f -- ${cur}) )
            return 0
            ;;
        --workers)
            # Suggest worker counts
            COMPREPLY=( $(compgen -W "1 2 4 8 16" -- ${cur}) )
            return 0
            ;;
    esac

    # Default to completing options
    COMPREPLY=( $(compgen -W "${global_opts}" -- ${cur}) )
    return 0
}

# Register the completion function
complete -F _only1mcp only1mcp
