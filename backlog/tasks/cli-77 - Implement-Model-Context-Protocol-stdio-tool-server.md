---
id: CLI-77
title: Implement Model Context Protocol stdio tool server
status: Done
assignee: []
created_date: '2026-11-19 14:30'
labels: []
milestone: m-20
dependencies:
  - CLI-73
ordinal: 77000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement the Model Context Protocol (MCP) tool server behind the `mcp` Cargo feature (decision-25). Inject `mycli mcp` command running a stdio JSON-RPC server implementing the MCP specification (`tools/list` and `tools/call`). Automatically expose controller commands as MCP tools with JSON input schemas derived from argument types, allowing direct consumption by AI agent environments (Claude Desktop, Cursor, Arena).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 mcp Cargo feature and stdio JSON-RPC protocol handler are implemented
- [x] #2 mycli mcp implements tools/list reflecting controller commands and argument schemas
- [x] #3 tools/call dispatches to controller command execution and returns structured results
- [x] #4 Integration tests verify MCP JSON-RPC protocol compliance and error handling over stdio
<!-- AC:END -->
