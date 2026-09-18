/**
 * Folders macOS gates behind a TCC consent prompt (Documents, Desktop, Downloads,
 * iCloud Drive). A well inside one of these may re-prompt for access — especially
 * for an unsigned/ad-hoc build whose signature changes per rebuild.
 */
const PROTECTED_SUBDIRS = ['Documents', 'Desktop', 'Downloads', 'Library/Mobile Documents'];

/** Whether `path` lives inside one of the home directory's macOS-protected folders. */
export function isProtectedLocation(path: string, home: string): boolean {
  if (!home) return false;
  return PROTECTED_SUBDIRS.some((sub) => {
    const base = `${home}/${sub}`;
    return path === base || path.startsWith(`${base}/`);
  });
}

/**
 * Cloud-synced roots (iCloud Drive, Dropbox, Google Drive, OneDrive, Box, and any
 * modern File-Provider mount under Library/CloudStorage). A mirror cloned into one
 * of these is silently re-uploaded to a third party — including the author's commit
 * identity and any secrets in its history — so M22 warns before the user picks one.
 */
const CLOUD_SYNCED_SUBDIRS = [
  'Library/Mobile Documents', // legacy iCloud Drive
  'Library/CloudStorage', // modern File-Provider mounts (iCloud/Dropbox/Drive/OneDrive/Box)
  'Dropbox',
  'Google Drive',
  'OneDrive',
];

/** Whether `path` lives inside a cloud-synced location that would re-upload a mirror (M22). */
export function isCloudSyncedLocation(path: string, home: string): boolean {
  if (!home) return false;
  return CLOUD_SYNCED_SUBDIRS.some((sub) => {
    const base = `${home}/${sub}`;
    return path === base || path.startsWith(`${base}/`);
  });
}
