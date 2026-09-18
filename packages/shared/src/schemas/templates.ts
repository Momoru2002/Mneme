import { z } from 'zod';

const ILLEGAL_RE = /[\\/:*?"<>|]/;

export const templateNameSchema = z
  .string()
  .min(1, 'Name is required')
  .max(128, 'Name must be at most 128 characters')
  .refine((s) => !ILLEGAL_RE.test(s), 'Name contains illegal characters')
  .refine((s) => !s.startsWith('.'), 'Name must not start with a dot')
  .refine((s) => s.trim() === s, 'Name must not start or end with whitespace');

export const templateCreateSchema = z.object({
  name: templateNameSchema,
  content: z.string().default(''),
});
export type TemplateCreateInput = z.infer<typeof templateCreateSchema>;

export const templateUpdateSchema = z.object({
  content: z.string(),
});
export type TemplateUpdateInput = z.infer<typeof templateUpdateSchema>;

export const applyTemplateSchema = z.object({
  vars: z.record(z.string(), z.string()).default({}),
});
export type ApplyTemplateInput = z.infer<typeof applyTemplateSchema>;
