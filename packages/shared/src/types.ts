import type { UserPrefsInput } from './schemas/settings.ts';

export type Well = {
  id: string;
  name: string;
  path: string;
  colorTag: string | null;
  sortOrder: number;
  // Unix-ms timestamps (Rust is the source of truth; serialized as i64).
  lastAccessedAt: number;
  createdAt: number;
  updatedAt: number;
};

export type WellListResponse = {
  wells: Well[];
  activeWellId: string | null;
};

export type WellValidationResult = {
  exists: boolean;
  isDirectory: boolean;
  readable: boolean;
  isObsidianWell: boolean;
  fileCount: number;
  error: string | null;
};

export type HostBrowseEntry = {
  name: string;
  path: string;
  hasChildren: boolean;
};

export type HostBrowseResponse = {
  parent: string | null;
  path: string;
  entries: HostBrowseEntry[];
};

export type TreeEntry = {
  name: string;
  path: string; // path relative to well root, e.g. "Coursera/Week 1.md"
  type: 'folder' | 'file';
  size: number | null;
  mtime: number; // Unix-ms
  hasChildren: boolean;
};

export type TreeResponse = {
  wellId: string;
  path: string;
  entries: TreeEntry[];
};

export type Frontmatter = Record<string, unknown>;

export type FileContent = {
  wellId: string;
  path: string;
  content: string; // markdown source (frontmatter NOT stripped)
  body: string; // markdown body (frontmatter stripped)
  frontmatter: Frontmatter;
  size: number;
  mtime: number; // Unix-ms
  hash: string; // sha256 hex of content (for conflict detection)
};

export type SaveFileResponse = {
  path: string;
  size: number;
  mtime: number; // Unix-ms
  hash: string;
};

export type CreateEntryResponse = {
  path: string;
  type: 'file' | 'folder';
};

export type Template = {
  name: string;
  path: string;
  content: string;
  size: number;
  mtime: number; // Unix-ms
};

export type TemplateListEntry = Pick<Template, 'name' | 'size' | 'mtime'>;

export type TemplateListResponse = {
  templates: TemplateListEntry[];
};

export type ApplyTemplateContext = {
  vars: Record<string, string>;
};

export type ApplyTemplateResult = {
  rendered: string;
};

/** Wire shape of user prefs — derived from the zod schema so the two never drift. */
export type UserPrefs = UserPrefsInput;

export type SettingsResponse = {
  prefs: UserPrefs;
};

export type SearchMatch =
  | { kind: 'filename' }
  | { kind: 'content'; line: number; column: number; snippet: string };

export type SearchResult = {
  path: string;
  matches: SearchMatch[];
};

export type SearchResponse = {
  results: SearchResult[];
};

export type SearchOptions = {
  caseSensitive: boolean;
  wholeWord: boolean;
  regex: boolean;
};
