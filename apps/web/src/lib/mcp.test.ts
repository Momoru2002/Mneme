import { describe, expect, it } from 'vitest';
import { claudeCodeCommand, claudeDesktopConfig, quotePath } from './mcp.ts';

describe('quotePath', () => {
  it('leaves a plain path untouched', () => {
    expect(quotePath('/usr/bin/mneme-mcp')).toBe('/usr/bin/mneme-mcp');
  });

  it('quotes a path containing spaces', () => {
    expect(quotePath('C:\\Program Files\\Mneme\\mneme-mcp.exe')).toBe(
      '"C:\\Program Files\\Mneme\\mneme-mcp.exe"',
    );
  });
});

describe('claudeCodeCommand', () => {
  it('builds the one-line add command', () => {
    expect(claudeCodeCommand('/usr/bin/mneme-mcp')).toBe(
      'claude mcp add --transport stdio mneme --scope user -- /usr/bin/mneme-mcp',
    );
  });
});

describe('claudeDesktopConfig', () => {
  it('emits valid JSON with a JSON-escaped Windows path', () => {
    const path = 'C:\\Program Files\\Mneme\\mneme-mcp.exe';
    const parsed = JSON.parse(claudeDesktopConfig(path));
    expect(parsed).toEqual({ mcpServers: { mneme: { command: path } } });
  });
});
