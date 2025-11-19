# Only1MCP Configuration Extension for VS Code

Provides rich language support for Only1MCP configuration files in Visual Studio Code.

## Features

### Syntax Highlighting
- Custom YAML syntax highlighting for Only1MCP configuration files
- Color-coded sections for easy visual parsing
- Highlights MCP servers, transports, and configuration options

### IntelliSense & Auto-Completion
- Smart suggestions for configuration options
- Auto-complete for transport types, algorithms, and settings
- Context-aware parameter suggestions
- Documentation on hover

### Snippets
- **14 built-in snippets** for common configurations:
  - `only1mcp-basic` - Complete basic configuration
  - `only1mcp-http` - HTTP transport
  - `only1mcp-stdio` - STDIO transport
  - `only1mcp-sse` - SSE transport
  - `only1mcp-ws` - WebSocket transport
  - `only1mcp-lb` - Load balancing
  - `only1mcp-cache` - Caching configuration
  - `only1mcp-ratelimit` - Rate limiting
  - `only1mcp-jwt` - JWT authentication
  - `only1mcp-oauth` - OAuth2 configuration
  - `only1mcp-admin` - Admin API
  - `only1mcp-metrics` - Metrics configuration
  - `only1mcp-tenant` - Multi-tenancy
  - `only1mcp-logging` - Logging setup

### Validation
- Real-time YAML syntax validation
- Schema validation for Only1MCP-specific configuration
- Error highlighting with helpful messages
- Quick fixes for common issues

### Commands
- **Validate Only1MCP Configuration** - Validate current configuration file
- **Generate Only1MCP Configuration** - Create new configuration from template

## Installation

### From VS Code Marketplace

1. Open VS Code
2. Press `Ctrl+P` (or `Cmd+P` on Mac)
3. Type `ext install only1mcp.only1mcp-config`
4. Press Enter

### From VSIX

1. Download the `.vsix` file
2. Open VS Code
3. Go to Extensions view (`Ctrl+Shift+X`)
4. Click `...` menu → Install from VSIX
5. Select the downloaded file

### Manual Installation

1. Copy the extension folder to:
   - **Windows**: `%USERPROFILE%\.vscode\extensions\`
   - **macOS/Linux**: `~/.vscode/extensions/`
2. Reload VS Code

## Usage

### Auto-Detection

The extension automatically activates for files matching:
- `only1mcp.yaml` or `only1mcp.yml`
- `*.only1mcp.yaml` or `*.only1mcp.yml`

### Snippets

Type a snippet prefix and press `Tab`:

```yaml
# Type 'only1mcp-basic' and press Tab
host: "0.0.0.0"
port: 8080

mcp_servers:
  - name: "my-server"
    transport:
      http:
        url: "http://localhost:3000"
...
```

### Validation

- Errors appear as red squiggly lines
- Hover over errors for details
- Use Quick Fix (`Ctrl+.`) for suggestions

### Commands

Access commands via:
- Command Palette (`Ctrl+Shift+P`)
- Right-click context menu
- Editor toolbar

## Configuration

Customize in VS Code settings (`settings.json`):

```json
{
  "[only1mcp-config]": {
    "editor.tabSize": 2,
    "editor.insertSpaces": true,
    "editor.formatOnSave": true,
    "editor.quickSuggestions": {
      "strings": true
    }
  }
}
```

## Supported Features

- ✅ Syntax highlighting
- ✅ Auto-completion
- ✅ Snippets (14 templates)
- ✅ Schema validation
- ✅ Error detection
- ✅ Hover documentation
- ✅ Command palette integration
- ✅ Context menu actions

## Requirements

- Visual Studio Code 1.80.0 or higher
- Only1MCP 0.6.0 or higher (for validation)

## Extension Settings

This extension contributes the following settings:

* `only1mcp.validation.enabled`: Enable/disable validation
* `only1mcp.completion.enabled`: Enable/disable auto-completion
* `only1mcp.snippets.enabled`: Enable/disable snippets

## Known Issues

- Schema validation requires YAML extension
- Real-time validation may have slight delay for large files

## Release Notes

### 0.6.0

Initial release with:
- Complete syntax highlighting
- 14 configuration snippets
- Schema validation
- Auto-completion support
- Command palette integration

## Contributing

Found a bug or have a feature request? Please file an issue on [GitHub](https://github.com/doublegate/Only1MCP/issues).

## License

GPL-3.0 - See LICENSE file for details

---

**Enjoy!** 🚀
