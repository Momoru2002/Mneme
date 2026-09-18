import { describe, expect, it } from 'vitest';
import { isCloudSyncedLocation, isProtectedLocation } from './macos-permissions.ts';

const HOME = '/Users/eco';

describe('isProtectedLocation', () => {
  it('flags macOS TCC-protected folders under home', () => {
    expect(isProtectedLocation('/Users/eco/Documents/Study', HOME)).toBe(true);
    expect(isProtectedLocation('/Users/eco/Desktop/notes', HOME)).toBe(true);
    expect(isProtectedLocation('/Users/eco/Downloads/x', HOME)).toBe(true);
    expect(isProtectedLocation('/Users/eco/Library/Mobile Documents/iCloud~md/v', HOME)).toBe(true);
    // the folder itself counts
    expect(isProtectedLocation('/Users/eco/Documents', HOME)).toBe(true);
  });

  it('does not flag unprotected locations', () => {
    expect(isProtectedLocation('/Users/eco/Mneme/Study', HOME)).toBe(false);
    expect(isProtectedLocation('/Users/eco/Projects/x', HOME)).toBe(false);
    // "Documents" under a different root is not the home-protected one
    expect(isProtectedLocation('/other/Documents/x', HOME)).toBe(false);
    // a sibling that merely shares a prefix segment
    expect(isProtectedLocation('/Users/eco/DocumentsArchive/x', HOME)).toBe(false);
    expect(isProtectedLocation('/Users/eco/Documents/Study', '')).toBe(false);
  });
});

describe('isCloudSyncedLocation (M22)', () => {
  it('flags cloud-synced roots that would re-upload a mirror', () => {
    expect(isCloudSyncedLocation('/Users/eco/Library/Mobile Documents/x', HOME)).toBe(true);
    expect(isCloudSyncedLocation('/Users/eco/Library/CloudStorage/Dropbox/x', HOME)).toBe(true);
    expect(isCloudSyncedLocation('/Users/eco/Library/CloudStorage/GoogleDrive-a/x', HOME)).toBe(
      true,
    );
    expect(isCloudSyncedLocation('/Users/eco/Dropbox/repo', HOME)).toBe(true);
    expect(isCloudSyncedLocation('/Users/eco/Google Drive/repo', HOME)).toBe(true);
    expect(isCloudSyncedLocation('/Users/eco/OneDrive', HOME)).toBe(true);
  });

  it('does not flag a plain local folder', () => {
    expect(isCloudSyncedLocation('/Users/eco/Mneme/mirrors/repo', HOME)).toBe(false);
    expect(isCloudSyncedLocation('/Users/eco/Projects/repo', HOME)).toBe(false);
    // a sibling sharing a prefix segment is not a match
    expect(isCloudSyncedLocation('/Users/eco/DropboxArchive/x', HOME)).toBe(false);
    expect(isCloudSyncedLocation('/Users/eco/Dropbox/x', '')).toBe(false);
  });
});
