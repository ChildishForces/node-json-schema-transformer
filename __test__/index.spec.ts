import { expect, test } from 'bun:test'

import {
  CollectionSession,
  defaultHelpersFile,
  extensionFor,
  helpersContent,
  jsonSchemaToKotlin,
  jsonSchemaToPydantic,
  jsonSchemaToRust,
  jsonSchemaToSwift,
  jsonSchemaToTypescript,
  jsonSchemaToZodModule,
  Language,
  transform,
} from '../index'

const personSchema = {
  type: 'object',
  properties: {
    id: { type: 'string', format: 'uuid' },
    name: { type: 'string' },
  },
  required: ['id'],
}

const uniqueItemsSchema = {
  type: 'array',
  items: { type: 'integer' },
  uniqueItems: true,
}

test('jsonSchemaToZodModule emits a complete module', () => {
  const module = jsonSchemaToZodModule(personSchema, 'person')
  expect(module).toContain('import { z } from "zod"')
  expect(module).toContain('export const PersonSchema')
  expect(module).toContain('export type Person')
  expect(module).toContain('z.string().uuid()')
  expect(module).toContain('name: z.string().optional()')
})

test('jsonSchemaToTypescript emits type definitions', () => {
  const output = jsonSchemaToTypescript(personSchema, 'person')
  expect(output).toContain('Person')
  expect(output).toContain('id')
})

test('jsonSchemaToPydantic emits a Pydantic model', () => {
  const output = jsonSchemaToPydantic(personSchema, 'person')
  expect(output).toContain('@pydantic_dataclass')
  expect(output).toContain('class Person:')
})

test('jsonSchemaToSwift emits a Codable struct', () => {
  const output = jsonSchemaToSwift(personSchema, 'person')
  expect(output).toContain('struct Person')
  expect(output).toContain('Codable')
})

test('jsonSchemaToKotlin emits a Kotlin class', () => {
  const output = jsonSchemaToKotlin(personSchema, 'person')
  expect(output).toContain('class Person')
  expect(output).toContain('kotlinx.serialization')
})

test('jsonSchemaToRust emits serde types', () => {
  const output = jsonSchemaToRust(personSchema, 'person')
  expect(output).toContain('pub struct Person')
})

test('transform accepts every Language value', () => {
  for (const language of [
    Language.Zod,
    Language.TypeScript,
    Language.Pydantic,
    Language.Swift,
    Language.Kotlin,
    Language.Rust,
  ]) {
    const output = transform(personSchema, language, { name: 'person' })
    expect(output.length).toBeGreaterThan(0)
  }
})

test('name falls back to the schema title', () => {
  const module = jsonSchemaToZodModule({ title: 'User Created', type: 'string' })
  expect(module).toContain('export const UserCreatedSchema')
})

test('explicit name beats the schema title', () => {
  const module = jsonSchemaToZodModule({ title: 'User Created', type: 'string' }, 'override')
  expect(module).toContain('export const OverrideSchema')
})

test('missing name is an error', () => {
  expect(() => jsonSchemaToZodModule({ type: 'string' })).toThrow('no name provided')
})

test('transform with mutable emits Swift var instead of let', () => {
  const schema = { type: 'object', properties: { a: { type: 'string' } }, required: ['a'] }
  expect(transform(schema, Language.Swift, { name: 'm', mutable: true })).toContain('var a')
  expect(transform(schema, Language.Swift, { name: 'm' })).toContain('let a')
})

test('transform with enforceFormats false drops format validation', () => {
  const schema = { type: 'string', format: 'email' }
  expect(transform(schema, Language.Zod, { name: 'e' })).toContain('.email()')
  expect(transform(schema, Language.Zod, { name: 'e', enforceFormats: false })).not.toContain('.email()')
})

test('transform with helpers references the shared file instead of inlining', () => {
  const output = transform(uniqueItemsSchema, Language.Zod, {
    name: 'x',
    helpers: { fileName: 'jst-helpers.ts' },
  })
  expect(output).toContain('from "./jst-helpers"')
})

test('transform resolves remote refs from the remotes registry', () => {
  const schema = {
    type: 'object',
    properties: { address: { $ref: 'https://example.com/address.json' } },
    unevaluatedProperties: false,
  }
  const remotes = {
    'https://example.com/address.json': {
      type: 'object',
      properties: { street: { type: 'string' } },
      required: ['street'],
    },
  }
  const output = transform(schema, Language.Zod, { name: 'person', remotes })
  expect(output.length).toBeGreaterThan(0)
})

test('CollectionSession tailors helpers to needed components', () => {
  const session = new CollectionSession(Language.Zod)
  const module = session.emit(uniqueItemsSchema, 'Sample')
  expect(module).toContain('from "./jst-helpers"')
  expect(session.helperNeeds()).toEqual(['stable_stringify'])
  const helpers = session.helpers()
  expect(helpers).not.toBeNull()
  expect(helpers!.fileName).toBe('jst-helpers.ts')
  expect(helpers!.content).toContain('_stableStringify')
  expect(helpers!.content).not.toContain('_deepEqual')
})

test('CollectionSession without helper needs produces no helpers file', () => {
  const session = new CollectionSession(Language.Zod)
  session.emit({ type: 'object', properties: { id: { type: 'string' } } }, 'Plain')
  expect(session.helperNeeds()).toEqual([])
  expect(session.helpers()).toBeNull()
})

test('CollectionSession honors a custom helpers file and dirPrefix', () => {
  const session = new CollectionSession(Language.Zod, { helpersFile: 'custom.ts' })
  const module = session.emit(uniqueItemsSchema, 'Nested', '../')
  expect(module).toContain('from "../custom"')
  expect(session.helpersFileName()).toBe('custom.ts')
})

test('extensionFor reports each language extension', () => {
  expect(extensionFor(Language.Zod)).toBe('zod.ts')
  expect(extensionFor(Language.TypeScript)).toBe('d.ts')
  expect(extensionFor(Language.Pydantic)).toBe('py')
  expect(extensionFor(Language.Swift)).toBe('swift')
  expect(extensionFor(Language.Kotlin)).toBe('kt')
  expect(extensionFor(Language.Rust)).toBe('rs')
})

test('defaultHelpersFile reports conventional names', () => {
  expect(defaultHelpersFile(Language.Zod)).toBe('jst-helpers.ts')
  expect(defaultHelpersFile(Language.TypeScript)).toBeNull()
  expect(defaultHelpersFile(Language.Pydantic)).toBe('jst_helpers.py')
  expect(defaultHelpersFile(Language.Swift)).toBe('JstHelpers.swift')
  expect(defaultHelpersFile(Language.Kotlin)).toBe('JstHelpers.kt')
  expect(defaultHelpersFile(Language.Rust)).toBe('jst_helpers.rs')
})

test('helpersContent returns the full helpers superset', () => {
  expect(helpersContent(Language.Zod)).not.toBeNull()
  expect(helpersContent(Language.TypeScript)).toBeNull()
})
