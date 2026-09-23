import { useQuery } from '@tanstack/react-query';
import { api } from './api-client.ts';
import { isTauri } from './web-mode.ts';

export const MCP_BINARY_KEY = ['mcp', 'binary'] as const;

/** Where the bundled `mneme-mcp` sidecar lives. Desktop-only. */
export function useMcpBinaryInfo() {
  return useQuery({
    queryKey: MCP_BINARY_KEY,
    queryFn: api.mcp.binaryInfo,
    enabled: isTauri(),
    staleTime: Number.POSITIVE_INFINITY,
  });
}

/** Quote a path for a POSIX shell / PowerShell command line when it contains spaces. */
export function quotePath(path: string): string {
  return /[\s"']/.test(path) ? `"${path.replace(/"/g, '\\"')}"` : path;
}

export function claudeCodeCommand(path: string): string {
  return `claude mcp add --transport stdio mneme --scope user -- ${quotePath(path)}`;
}

/** The `mcpServers` snippet for Claude Desktop's `claude_desktop_config.json`. */
export function claudeDesktopConfig(path: string): string {
  return JSON.stringify({ mcpServers: { mneme: { command: path } } }, null, 2);
}
