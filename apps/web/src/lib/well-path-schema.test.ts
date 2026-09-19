import { relativePathSchema, wellPathSchema } from '@mneme/shared';
import { describe, expect, it } from 'vitest';

// Regression coverage for a real bug: wellPathSchema originally checked only
// `startsWith('/')` (POSIX), so a Windows path like "D:\Mneme\" pasted into
// the Add Well dialog was rejected client-side as "not absolute" before the
// request ever reached the (correctly cross-platform) Rust backend.
describe('wellPathSchema', () => {
  it.each([
    'D:\\Mneme\\',
    'D:/Mneme/',
    'C:\\Users\\bob\\Documents\\vault',
    '\\\\server\\share\\notes',
    '//server/share/notes',
    '/home/user/notes',
    '/Users/bob/Documents/vault',
  ])('accepts absolute path %s', (p) => {
    expect(wellPathSchema.safeParse(p).success).toBe(true);
  });

  it.each(['notes/subfolder', 'D:notes', '', 'relative\\path', '../escape'])(
    'rejects non-absolute path %s',
    (p) => {
      expect(wellPathSchema.safeParse(p).success).toBe(false);
    },
  );
});

describe('relativePathSchema', () => {
  it.each(['notes/subfolder.md', 'a/b/c.md', 'file.md'])('accepts relative path %s', (p) => {
    expect(relativePathSchema.safeParse(p).success).toBe(true);
  });

  it.each([
    '/etc/passwd',
    'D:\\Windows\\System32',
    'C:/Windows/System32',
    '\\\\server\\share\\file',
    '//server/share/file',
  ])('rejects absolute path %s on every platform', (p) => {
    expect(relativePathSchema.safeParse(p).success).toBe(false);
  });
});
