import { z } from 'zod';

export const wellNameSchema = z
  .string()
  .min(1, 'Name is required')
  .max(64, 'Name must be at most 64 characters');

// An absolute path on ANY of the platforms Mneme runs on:
// - POSIX (macOS/Linux): starts with "/"
// - Windows drive letter: "C:\", "C:/", or bare "C:" followed by a separator
// - Windows UNC: "\\server\share..." (or the forward-slash form some tools
//   accept, "//server/share...")
// A bare "startsWith('/')" check (POSIX-only) rejects every valid Windows
// path a user could paste or get from Explorer, which is the actual bug this
// was written to fix — the backend's own check (Rust's Path::is_absolute(),
// already correctly cross-platform) never even got a chance to see the path.
const WINDOWS_DRIVE_ABSOLUTE = /^[a-zA-Z]:[\\/]/;
const WINDOWS_UNC = /^(\\\\|\/\/)/;

function isAbsolutePathAnyPlatform(s: string): boolean {
  return s.startsWith('/') || WINDOWS_DRIVE_ABSOLUTE.test(s) || WINDOWS_UNC.test(s);
}

export const wellPathSchema = z
  .string()
  .min(1, 'Path is required')
  .refine(isAbsolutePathAnyPlatform, 'Path must be absolute');

export const addWellInputSchema = z.object({
  name: wellNameSchema,
  path: wellPathSchema,
  colorTag: z
    .string()
    .regex(/^#[0-9a-fA-F]{6}$/, 'Color must be a hex like #4FD1C5')
    .nullable()
    .optional(),
});
export type AddWellInput = z.infer<typeof addWellInputSchema>;

export const updateWellInputSchema = z.object({
  name: wellNameSchema.optional(),
  colorTag: z
    .string()
    .regex(/^#[0-9a-fA-F]{6}$/, 'Color must be a hex like #4FD1C5')
    .nullable()
    .optional(),
});
export type UpdateWellInput = z.infer<typeof updateWellInputSchema>;

export const validateWellInputSchema = z.object({
  path: wellPathSchema,
});
export type ValidateWellInput = z.infer<typeof validateWellInputSchema>;
