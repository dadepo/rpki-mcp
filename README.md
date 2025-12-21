# rpki-mcp

An MCP server that exposes functionalities of RPKI relying parties.

## Building

```bash
cargo build --release
```

## Running

The server requires an RPKI relying party endpoint as a command-line argument:

```bash
cargo run --release <endpoint>
```

For example:

```bash
cargo run --release http://127.0.0.1:8323
```

## Tools

The server provides the following tools:

- `status`: Returns the status of the RPKI relying party, including version, serial number, and update information.

- `validity`: Validates a route announcement by checking if it is RPKI valid, invalid, or not found. Requires an ASN and IP prefix as parameters. Returns the validation result along with the complete set of Validated ROA Payloads (VRPs) that determined the outcome.

- `roas`: Retrieves all Route Origin Authorizations (ROAs) for a given Autonomous System Number (ASN). Requires an ASN as a parameter. Returns a JSON object containing metadata and a list of ROAs associated with the specified ASN.

## Configuration

To use this MCP server with a chat client, you need to configure it in your client's settings.

### Claude Desktop

Add the following to your Claude Desktop configuration file (typically located at `~/Library/Application Support/Claude/claude_desktop_config.json` on macOS, or `%APPDATA%\Claude\claude_desktop_config.json` on Windows):

```json
{
  "mcpServers": {
    "rpki-mcp": {
      "command": "target/release/rpki-mcp",
      "args": ["http://127.0.0.1:8323"]
    }
  }
}
```

Replace `target/release/rpki-mcp` with the absolute path to the built binary if needed, and update the endpoint URL to match your RPKI relying party server.

## Logging

Logs are written to `logs/rpki_mcp.log`.
