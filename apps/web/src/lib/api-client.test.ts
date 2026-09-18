import { afterEach, describe, expect, it, vi } from 'vitest';

// Mock the Tauri invoke boundary. Every api-client method must funnel through
// this single call; the test asserts the exact command name + argument shape so
// a typo in any of the 40 commands is caught here (a green query suite would
// not catch a wrong command name — it just 404s at runtime).
const invoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invoke(...args),
}));

// Default: act as if we're inside Tauri (desktop). Tests that exercise the
// browser/fetch path override this per-test via vi.mocked(isTauri).
vi.mock('./web-mode.ts', () => ({
  isTauri: vi.fn(() => true),
  getWebToken: vi.fn(() => null),
  bootstrapToken: vi.fn(),
  clearWebToken: vi.fn(),
  notifyWebAuthExpired: vi.fn(),
}));

import { ApiError, api } from './api-client.ts';
import { clearWebToken, getWebToken, isTauri, notifyWebAuthExpired } from './web-mode.ts';

afterEach(() => {
  invoke.mockReset();
  vi.mocked(isTauri).mockReturnValue(true);
  vi.mocked(getWebToken).mockReturnValue(null);
});

/** Arrange invoke to resolve with `value`, run `fn`, assert the invoke call. */
async function expectInvoke(
  fn: () => Promise<unknown>,
  command: string,
  args: Record<string, unknown>,
  resolved: unknown = {},
): Promise<void> {
  invoke.mockResolvedValueOnce(resolved);
  await fn();
  expect(invoke).toHaveBeenCalledTimes(1);
  expect(invoke).toHaveBeenCalledWith(command, args);
}

describe('api-client → invoke command mapping', () => {
  it('wells', async () => {
    await expectInvoke(() => api.wells.list(), 'wells_list', {});
    invoke.mockReset();
    await expectInvoke(() => api.wells.get('id1'), 'wells_get', { id: 'id1' });
    invoke.mockReset();
    const addInput = { name: 'W', path: '/x', colorTag: null };
    await expectInvoke(() => api.wells.add(addInput), 'wells_add', { input: addInput });
    invoke.mockReset();
    const upd = { name: 'W2' };
    await expectInvoke(() => api.wells.update('id1', upd), 'wells_update', {
      id: 'id1',
      input: upd,
    });
    invoke.mockReset();
    await expectInvoke(() => api.wells.remove('id1'), 'wells_remove', { id: 'id1' });
    invoke.mockReset();
    await expectInvoke(() => api.wells.activate('id1'), 'wells_activate', { id: 'id1' });
    invoke.mockReset();
    await expectInvoke(() => api.wells.validate('/x'), 'wells_validate', { path: '/x' });
    invoke.mockReset();
    await expectInvoke(() => api.wells.browse('/x'), 'wells_browse_host', { path: '/x' });
    invoke.mockReset();
    await expectInvoke(() => api.wells.hostHome(), 'wells_host_home', {});
  });

  it('tree', async () => {
    await expectInvoke(() => api.tree.list('w', 'sub'), 'tree_list', { wellId: 'w', path: 'sub' });
    invoke.mockReset();
    await expectInvoke(() => api.tree.list('w'), 'tree_list', { wellId: 'w', path: '' });
  });

  it('files', async () => {
    await expectInvoke(() => api.files.read('w', 'a.md'), 'files_read', {
      wellId: 'w',
      path: 'a.md',
    });
    invoke.mockReset();
    const ci = { path: 'a.md', content: 'x' };
    await expectInvoke(() => api.files.create('w', ci), 'files_create', { wellId: 'w', input: ci });
    invoke.mockReset();
    const ui = { path: 'a.md', content: 'y', expectedHash: 'h' };
    await expectInvoke(() => api.files.update('w', ui), 'files_update', { wellId: 'w', input: ui });
    invoke.mockReset();
    await expectInvoke(() => api.files.remove('w', 'a.md'), 'files_remove', {
      wellId: 'w',
      path: 'a.md',
    });
    invoke.mockReset();
    const ri = { oldPath: 'a.md', newPath: 'b.md' };
    await expectInvoke(() => api.files.rename('w', ri), 'files_rename', { wellId: 'w', input: ri });
    invoke.mockReset();
    const mi = { sourcePath: 'a.md', destPath: 'c/b.md' };
    await expectInvoke(() => api.files.move('w', mi), 'files_move', { wellId: 'w', input: mi });
    invoke.mockReset();
    const di = { path: 'a.md' };
    await expectInvoke(() => api.files.duplicate('w', di), 'files_duplicate', {
      wellId: 'w',
      input: di,
    });
  });

  it('folders', async () => {
    const ci = { path: 'NewFolder' };
    await expectInvoke(() => api.folders.create('w', ci), 'folders_create', {
      wellId: 'w',
      input: ci,
    });
    invoke.mockReset();
    await expectInvoke(() => api.folders.remove('w', 'Old', true), 'folders_remove', {
      wellId: 'w',
      input: { path: 'Old', confirm: true },
    });
    invoke.mockReset();
    const ri = { oldPath: 'A', newPath: 'B' };
    await expectInvoke(() => api.folders.rename('w', ri), 'folders_rename', {
      wellId: 'w',
      input: ri,
    });
    invoke.mockReset();
    const mi = { sourcePath: 'A', destPath: 'C/A' };
    await expectInvoke(() => api.folders.move('w', mi), 'folders_move', { wellId: 'w', input: mi });
  });

  it('templates', async () => {
    await expectInvoke(() => api.templates.list(), 'templates_list', {});
    invoke.mockReset();
    await expectInvoke(() => api.templates.get('daily'), 'templates_get', { name: 'daily' });
    invoke.mockReset();
    const ci = { name: 'daily', content: '# {{date}}' };
    await expectInvoke(() => api.templates.create(ci), 'templates_create', { input: ci });
    invoke.mockReset();
    const ui = { content: 'x' };
    await expectInvoke(() => api.templates.update('daily', ui), 'templates_update', {
      name: 'daily',
      input: ui,
    });
    invoke.mockReset();
    await expectInvoke(() => api.templates.remove('daily'), 'templates_remove', { name: 'daily' });
    invoke.mockReset();
    await expectInvoke(() => api.templates.duplicate('daily'), 'templates_duplicate', {
      name: 'daily',
    });
    invoke.mockReset();
    const ai = { vars: { x: '1' } };
    await expectInvoke(() => api.templates.apply('daily', ai), 'templates_apply', {
      name: 'daily',
      input: ai,
    });
    invoke.mockReset();
    await expectInvoke(
      () => api.templates.importDefaults(),
      'templates_import_defaults',
      {},
      { created: [] },
    );
  });

  it('settings (param key is `patch`, not `input`)', async () => {
    await expectInvoke(() => api.settings.get(), 'settings_get', {});
    invoke.mockReset();
    const patch = { fontSize: 16 };
    await expectInvoke(() => api.settings.update(patch), 'settings_update', { patch });
  });

  it('search', async () => {
    await expectInvoke(
      () => api.search.query('w', 'term', false),
      'search_query',
      { wellId: 'w', q: 'term', includeContent: false, options: null },
      { results: [] },
    );
    invoke.mockReset();
    await expectInvoke(
      () => api.search.query('w', 'term'),
      'search_query',
      { wellId: 'w', q: 'term', includeContent: true, options: null },
      { results: [] },
    );
    invoke.mockReset();
    const o = { caseSensitive: true, wholeWord: false, regex: true };
    await expectInvoke(
      () => api.search.query('w', 'term', true, o),
      'search_query',
      { wellId: 'w', q: 'term', includeContent: true, options: o },
      { results: [] },
    );
  });
});

describe('api-client error model', () => {
  it('wraps an invoke string rejection in an ApiError carrying the message', async () => {
    invoke.mockRejectedValueOnce('file not found');
    await expect(api.files.read('w', 'x.md')).rejects.toBeInstanceOf(ApiError);
    invoke.mockRejectedValueOnce('file not found');
    await expect(api.files.read('w', 'x.md')).rejects.toMatchObject({
      message: 'file not found',
      code: 'not_found',
    });
  });

  it('classifies sanitized error strings into codes', async () => {
    invoke.mockRejectedValueOnce('invalid credentials');
    await expect(api.wells.list()).rejects.toMatchObject({
      code: 'unknown',
      message: 'invalid credentials',
    });
  });

  it('classifies the hash-mismatch message as a conflict (replaces HTTP 409)', async () => {
    invoke.mockRejectedValueOnce('file was changed by another process');
    await expect(api.files.update('w', { path: 'a.md', content: 'x' })).rejects.toMatchObject({
      code: 'conflict',
    });
  });

  it('ApiError is a real Error so `err instanceof Error ? err.message` keeps working', async () => {
    invoke.mockRejectedValueOnce('boom');
    const err = await api.wells.list().catch((e) => e);
    expect(err).toBeInstanceOf(Error);
    expect(err.message).toBe('boom');
  });
});

describe('api-client → fetch transport (web mode)', () => {
  afterEach(() => {
    vi.restoreAllMocks();
    // restoreAllMocks does NOT undo vi.stubGlobal — clear the stubbed `fetch` so
    // it can't leak into a later test.
    vi.unstubAllGlobals();
    // Reset back to Tauri mode after each fetch-path test.
    vi.mocked(isTauri).mockReturnValue(true);
    vi.mocked(getWebToken).mockReturnValue(null);
    vi.mocked(clearWebToken).mockReset();
    vi.mocked(notifyWebAuthExpired).mockReset();
  });

  it('issues a POST to /api/<cmd> with Authorization header and JSON body', async () => {
    vi.mocked(isTauri).mockReturnValue(false);
    vi.mocked(getWebToken).mockReturnValue('deadbeef');

    const mockFetch = vi
      .fn()
      .mockResolvedValueOnce(new Response(JSON.stringify({ results: [] }), { status: 200 }));
    vi.stubGlobal('fetch', mockFetch);

    await api.search.query('w1', 'hello');

    expect(mockFetch).toHaveBeenCalledTimes(1);
    const [url, init] = mockFetch.mock.calls[0] as [string, RequestInit];
    expect(url).toBe('/api/search_query');
    expect((init.headers as Record<string, string>).authorization).toBe('Bearer deadbeef');
    expect(JSON.parse(init.body as string)).toEqual({
      wellId: 'w1',
      q: 'hello',
      includeContent: true,
      options: null,
    });
  });

  it('throws ApiError(not_found) on 404', async () => {
    vi.mocked(isTauri).mockReturnValue(false);
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValueOnce(new Response('not found', { status: 404 })),
    );

    await expect(api.wells.list()).rejects.toMatchObject({
      code: 'not_found',
    });
  });

  it('throws ApiError(unknown) on non-404 error', async () => {
    vi.mocked(isTauri).mockReturnValue(false);
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValueOnce(new Response('server error', { status: 500 })),
    );

    await expect(api.wells.list()).rejects.toMatchObject({
      code: 'unknown',
      message: 'server error',
    });
  });

  it('throws ApiError(unauthorized) on 401 and clears + signals the expired token', async () => {
    vi.mocked(isTauri).mockReturnValue(false);
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValueOnce(new Response('missing or invalid token', { status: 401 })),
    );

    const err = await api.wells.list().catch((e) => e);
    expect(err).toBeInstanceOf(ApiError);
    expect(err).toMatchObject({ code: 'unauthorized', message: 'missing or invalid token' });

    // The 401 path wipes the stale token and signals the root overlay.
    expect(clearWebToken).toHaveBeenCalled();
    expect(notifyWebAuthExpired).toHaveBeenCalled();
  });

  it('returns null for empty-body (void) responses', async () => {
    vi.mocked(isTauri).mockReturnValue(false);
    vi.stubGlobal('fetch', vi.fn().mockResolvedValueOnce(new Response('', { status: 200 })));

    const result = await api.wells.remove('id1');
    expect(result).toBeNull();
  });
});
