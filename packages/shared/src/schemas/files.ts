import { z } from 'zod';

// Filename validation: no path separators / control chars / reserved chars
const ILLEGAL_RE = /[\\/:*?"<>| -]/;

export const filenameSchema = z
  .string()
  .min(1, 'Name is required')
  .max(255, 'Name must be at most 255 characters')
  .refine((s) => !ILLEGAL_RE.test(s), 'Name contains illegal characters (\\/:*?"<>|)')
  .refine((s) => s.trim() === s, 'Name must not start or end with whitespace')
  .refine((s) => !s.startsWith('.'), 'Name must not start with a dot')
  .refine((s) => s !== '.' && s !== '..', 'Reserved name');

export const relativePathSchema = z
  .string()
  .min(1, 'Path is required')
  .refine((s) => !s.includes('\0'), 'Path contains null byte')
  .refine((s) => !s.startsWith('/'), 'Path must be relative to well root');

export const wellIdQuerySchema = z.object({
  wellId: z.string().uuid(),
});

export const fileCreateSchema = z.object({
  path: relativePathSchema,
  content: z.string().default(''),
});
export type FileCreateInput = z.infer<typeof fileCreateSchema>;

export const fileUpdateSchema = z.object({
  path: relativePathSchema,
  content: z.string(),
  expectedHash: z.string().optional(),
});
export type FileUpdateInput = z.infer<typeof fileUpdateSchema>;

export const renameSchema = z.object({
  oldPath: relativePathSchema,
  newPath: relativePathSchema,
});
export type RenameInput = z.infer<typeof renameSchema>;

export const moveSchema = z.object({
  sourcePath: relativePathSchema,
  destPath: relativePathSchema,
});
export type MoveInput = z.infer<typeof moveSchema>;

export const duplicateSchema = z.object({
  path: relativePathSchema,
});
export type DuplicateInput = z.infer<typeof duplicateSchema>;

export const folderCreateSchema = z.object({
  path: relativePathSchema,
});
export type FolderCreateInput = z.infer<typeof folderCreateSchema>;
