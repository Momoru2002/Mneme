import { z } from 'zod';

export const wellNameSchema = z
  .string()
  .min(1, 'Name is required')
  .max(64, 'Name must be at most 64 characters');

export const wellPathSchema = z
  .string()
  .min(1, 'Path is required')
  .refine((s) => s.startsWith('/'), 'Path must be absolute');

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
