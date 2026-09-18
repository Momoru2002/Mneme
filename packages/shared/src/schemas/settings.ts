import { z } from 'zod';

export const userPrefsSchema = z.object({
  theme: z.string(),
  autoSaveMs: z.number().int().min(500).max(60_000),
  fontSize: z.number().int().min(10).max(24),
  tabWidth: z.number().int().min(2).max(8),
  lineNumbers: z.boolean(),
  previewMermaid: z.boolean(),
  treeFontSize: z.number().int().min(11).max(16),
  wordWrap: z.boolean(),
  indentStyle: z.enum(['tabs', 'spaces']),
  accentColor: z.string().nullable(),
  goldColor: z.string().nullable(),
  allowNetwork: z.boolean().default(false),
});
export type UserPrefsInput = z.infer<typeof userPrefsSchema>;

export const userPrefsPatchSchema = userPrefsSchema.partial();
export type UserPrefsPatch = z.infer<typeof userPrefsPatchSchema>;

export const DEFAULT_PREFS: UserPrefsInput = {
  theme: 'wellspring-dark',
  autoSaveMs: 2000,
  fontSize: 14,
  tabWidth: 2,
  lineNumbers: true,
  previewMermaid: true,
  treeFontSize: 13,
  wordWrap: true,
  indentStyle: 'spaces',
  accentColor: null,
  goldColor: null,
  allowNetwork: false,
};
