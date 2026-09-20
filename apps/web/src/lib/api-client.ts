import type {
  AddWellInput,
  ApplyTemplateInput,
  ApplyTemplateResult,
  DuplicateInput,
  FileContent,
  FileCreateInput,
  FileUpdateInput,
  FolderCreateInput,
  HostBrowseResponse,
  MoveInput,
  RenameInput,
  SaveFileResponse,
  SearchOptions,
  SearchResponse,
  SettingsResponse,
  Template,
  TemplateCreateInput,
  TemplateListEntry,
  TemplateListResponse,
  TemplateUpdateInput,
  TreeResponse,
  UpdateWellInput,
  UserPrefsPatch,
  Well,
  WellListResponse,
  WellValidationResult,
} from '@mneme/shared';
import { invoke } from '@tauri-apps/api/core';
import { clearWebToken, getWebToken, isTauri, notifyWebAuthExpired } from './web-mode.ts';

/**
 * Slice 7: every API call is an in-process Tauri `invoke` to a Rust
 * `#[tauri::command]` — no HTTP, no sidecar. Argument keys are camelCase; Tauri
 * maps them to the command's snake_case parameters. A command that takes an
 * `input: SomeInput` struct receives it under the `input` key (settings is the
 * exception — its parameter is named `patch`).
 */

/** Stable error codes derived from the Rust commands' sanitized messages. */
export type ApiErrorCode = 'not_found' | 'conflict' | 'unauthorized' | 'unknown';

/**
 * A failed command. Tauri rejects an `invoke` promise with the command's `Err`
 * value — here always a `String` (`client_message()` on the Rust side). We wrap
 * it so the rest of the app can keep using `err instanceof Error ? err.message`
 * and can branch on a stable `code` instead of the old HTTP status.
 */
export class ApiError extends Error {
  constructor(
    message: string,
    public code: ApiErrorCode,
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

function classify(message: string): ApiErrorCode {
  const m = message.toLowerCase();
  if (m.includes('not found')) return 'not_found';
  // Optimistic-concurrency conflict: files_update with a stale expectedHash
  // (FilesError::HashMismatch). Replaces the old HTTP 409.
  if (m === 'file was changed by another process') return 'conflict';
  return 'unknown';
}

/** The single invoke chokepoint: call a command, normalize its rejection. */
async function call<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  if (isTauri()) {
    // Desktop path: Tauri IPC invoke.
    try {
      return await invoke<T>(command, args);
    } catch (raw) {
      const message =
        typeof raw === 'string' ? raw : raw instanceof Error ? raw.message : 'Unexpected error';
      throw new ApiError(message, classify(message));
    }
  }

  // Browser path (web mode): HTTP fetch to the local server.
  const res = await fetch(`/api/${command}`, {
    method: 'POST',
    headers: {
      'content-type': 'application/json',
      authorization: `Bearer ${getWebToken() ?? ''}`,
    },
    body: JSON.stringify(args),
  });
  if (!res.ok) {
    const msg = await res.text();
    if (res.status === 401) {
      // Token missing/expired: drop it and signal the root overlay so the user
      // gets an actionable message instead of a silent empty screen.
      clearWebToken();
      notifyWebAuthExpired();
      throw new ApiError(msg || 'web access token missing or expired', 'unauthorized');
    }
    throw new ApiError(
      msg || `request failed (${res.status})`,
      res.status === 404 ? 'not_found' : 'unknown',
    );
  }
  // Commands that return nothing send an empty body; handle gracefully.
  const text = await res.text();
  return (text ? JSON.parse(text) : null) as T;
}

export const api = {
  wells: {
    list: (): Promise<WellListResponse> => call<WellListResponse>('wells_list'),
    get: (id: string): Promise<Well> => call<Well>('wells_get', { id }),
    add: (input: AddWellInput): Promise<Well> => call<Well>('wells_add', { input }),
    update: (id: string, input: UpdateWellInput): Promise<Well> =>
      call<Well>('wells_update', { id, input }),
    remove: (id: string): Promise<void> => call<void>('wells_remove', { id }),
    activate: (id: string): Promise<void> => call<void>('wells_activate', { id }),
    validate: (path: string): Promise<WellValidationResult> =>
      call<WellValidationResult>('wells_validate', { path }),
    // Both browse + hostHome return a full HostBrowseResponse (parent/path/entries).
    browse: (path: string): Promise<HostBrowseResponse> =>
      call<HostBrowseResponse>('wells_browse_host', { path }),
    hostHome: (): Promise<HostBrowseResponse> => call<HostBrowseResponse>('wells_host_home'),
  },

  tree: {
    list: (wellId: string, path = ''): Promise<TreeResponse> =>
      call<TreeResponse>('tree_list', { wellId, path }),
  },

  files: {
    read: (wellId: string, path: string): Promise<FileContent> =>
      call<FileContent>('files_read', { wellId, path }),
    create: (wellId: string, input: FileCreateInput): Promise<SaveFileResponse> =>
      call<SaveFileResponse>('files_create', { wellId, input }),
    update: (wellId: string, input: FileUpdateInput): Promise<SaveFileResponse> =>
      call<SaveFileResponse>('files_update', { wellId, input }),
    remove: (wellId: string, path: string): Promise<void> =>
      call<void>('files_remove', { wellId, path }),
    rename: (wellId: string, input: RenameInput): Promise<{ oldPath: string; newPath: string }> =>
      call<{ oldPath: string; newPath: string }>('files_rename', { wellId, input }),
    move: (wellId: string, input: MoveInput): Promise<{ sourcePath: string; destPath: string }> =>
      call<{ sourcePath: string; destPath: string }>('files_move', { wellId, input }),
    duplicate: (wellId: string, input: DuplicateInput): Promise<SaveFileResponse> =>
      call<SaveFileResponse>('files_duplicate', { wellId, input }),
  },

  folders: {
    create: (wellId: string, input: FolderCreateInput): Promise<{ path: string }> =>
      call<{ path: string }>('folders_create', { wellId, input }),
    remove: (wellId: string, path: string, confirm = true): Promise<void> =>
      call<void>('folders_remove', { wellId, input: { path, confirm } }),
    rename: (wellId: string, input: RenameInput): Promise<{ oldPath: string; newPath: string }> =>
      call<{ oldPath: string; newPath: string }>('folders_rename', { wellId, input }),
    move: (wellId: string, input: MoveInput): Promise<{ sourcePath: string; destPath: string }> =>
      call<{ sourcePath: string; destPath: string }>('folders_move', { wellId, input }),
  },

  templates: {
    list: (): Promise<TemplateListResponse> => call<TemplateListResponse>('templates_list'),
    get: (name: string): Promise<Template> => call<Template>('templates_get', { name }),
    create: (input: TemplateCreateInput): Promise<Template> =>
      call<Template>('templates_create', { input }),
    update: (name: string, input: TemplateUpdateInput): Promise<Template> =>
      call<Template>('templates_update', { name, input }),
    remove: (name: string): Promise<{ ok: boolean }> =>
      call<{ ok: boolean }>('templates_remove', { name }),
    duplicate: (name: string): Promise<Template> => call<Template>('templates_duplicate', { name }),
    apply: (name: string, input: ApplyTemplateInput): Promise<ApplyTemplateResult> =>
      call<ApplyTemplateResult>('templates_apply', { name, input }),
    importDefaults: (): Promise<{ created: TemplateListEntry[] }> =>
      call<{ created: TemplateListEntry[] }>('templates_import_defaults'),
  },

  settings: {
    get: (): Promise<SettingsResponse> => call<SettingsResponse>('settings_get'),
    // The Rust parameter is named `patch`, not `input`.
    update: (patch: UserPrefsPatch): Promise<SettingsResponse> =>
      call<SettingsResponse>('settings_update', { patch }),
  },

  search: {
    query: (
      wellId: string,
      q: string,
      includeContent = true,
      options?: SearchOptions,
    ): Promise<SearchResponse> =>
      call<SearchResponse>('search_query', { wellId, q, includeContent, options: options ?? null }),
  },

  system: {
    openPrivacySettings: (): Promise<void> => call<void>('open_macos_privacy_settings'),
    revealLogs: (): Promise<void> => call<void>('reveal_logs'),
    exportDiagnostics: (): Promise<string> => call<string>('copy_diagnostic_bundle'),
    uninstallAndWipe: (): Promise<void> => call<void>('uninstall_and_wipe'),
  },

  /** Tauri-only — the app-lock. Not reachable via web mode's HTTP dispatch by
   * design: unlocking requires being at the desktop app itself. */
  auth: {
    status: (): Promise<AuthStatus> => call<AuthStatus>('auth_status'),
    setPassword: (password: string): Promise<void> => call<void>('auth_set_password', { password }),
    unlock: (password: string): Promise<void> => call<void>('auth_unlock', { password }),
    lock: (): Promise<void> => call<void>('auth_lock'),
    changePassword: (currentPassword: string, newPassword: string): Promise<void> =>
      call<void>('auth_change_password', { currentPassword, newPassword }),
  },

  /** Tauri-only commands — only invoke these from desktop context (isTauri()). */
  webMode: {
    status: (): Promise<WebModeInfo> => call<WebModeInfo>('web_mode_status'),
    /** `lan: true` also allows other devices on this network (e.g. your phone) to connect. */
    enable: (lan = false): Promise<WebModeInfo> => call<WebModeInfo>('web_mode_enable', { lan }),
    disable: (): Promise<void> => call<void>('web_mode_disable'),
    openBrowser: (): Promise<void> => call<void>('web_mode_open_browser'),
  },
};

export interface AuthStatus {
  /** Whether a password has ever been set. `false` → show a "create a password" screen. */
  isSetUp: boolean;
  unlocked: boolean;
}

export interface WebModeInfo {
  running: boolean;
  url: string | null;
  /** Reachable from other devices on the same local network — only set when LAN mode is on. */
  lanUrl: string | null;
}
