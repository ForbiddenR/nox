# Agent Script Examples

This directory contains example agent scripts that work with the real runner in the Cox desktop app.

## How Agent Scripts Work

The real runner executes a command and captures its output as structured events. Your script should:

1. **Read the prompt from stdin** - The user's request is provided as a single line
2. **Output JSON events to stdout** - One JSON object per line
3. **Exit with code 0 on success** - Or non-zero on failure

## Event Format

Each line of stdout should be a JSON object with this structure:

```json
{"type": "text", "data": "Your message here"}
{"type": "tool_start", "data": {"tool": "tool_name", "args": {...}}}
{"type": "tool_end", "data": {"tool": "tool_name", "result": "Result description"}}
{"type": "error", "data": "Error message"}
```

### Event Types

- **text** - Display text to the user (supports markdown)
  - `data`: string - The text to display
  
- **tool_start** - Indicate a tool is being called
  - `data.tool`: string - Tool name
  - `data.args`: object - Tool arguments (optional)
  
- **tool_end** - Indicate a tool call completed
  - `data.tool`: string - Tool name
  - `data.result`: string - Result description
  
- **error** - Report an error
  - `data`: string - Error message

## Example Usage

The `example-agent.sh` script demonstrates the basic pattern:

```bash
./example-agent.sh
```

When run by the desktop app, it will:
1. Receive the user's prompt via stdin
2. Output a sequence of text and tool events
3. Exit successfully

## Creating Your Own Agent

You can create an agent in any language that can:
- Read from stdin
- Write JSON lines to stdout
- Execute as a subprocess

### Python Example

```python
#!/usr/bin/env python3
import json
import sys

# Read prompt
prompt = input()

# Output events
print(json.dumps({"type": "text", "data": f"Processing: {prompt}"}))
print(json.dumps({"type": "tool_start", "data": {"tool": "analyze", "args": {}}}))
print(json.dumps({"type": "tool_end", "data": {"tool": "analyze", "result": "Complete"}}))
print(json.dumps({"type": "text", "data": "Done!"}))
```

### Node.js Example

```javascript
#!/usr/bin/env node
const readline = require('readline');

const rl = readline.createInterface({
  input: process.stdin,
  output: process.stdout,
  terminal: false
});

rl.on('line', (prompt) => {
  console.log(JSON.stringify({type: 'text', data: `Processing: ${prompt}`}));
  console.log(JSON.stringify({type: 'tool_start', data: {tool: 'analyze', args: {}}}));
  console.log(JSON.stringify({type: 'tool_end', data: {tool: 'analyze', result: 'Complete'}}));
  console.log(JSON.stringify({type: 'text', data: 'Done!'}));
});
```

## Integration with Desktop App

To use your custom agent script in the Cox desktop app, you'll need to configure it to use `start_run_real` instead of `start_run_mock` in the backend. This is currently set to mock mode by default for testing.

Future versions will allow configuring the agent command via UI or settings file.
