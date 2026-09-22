# `node-json-schema-transformer`

Node bindings for [json-schema-transformer](https://github.com/ChildishForces/json-schema-transformer) — transform JSON Schema into native types and validation schemas for multiple languages, powered by Rust via [napi-rs](https://napi.rs).

Supported targets: **Zod** (TypeScript), **TypeScript** declarations, **Pydantic** (Python), **Swift** (Codable), **Kotlin** (kotlinx.serialization), and **Rust** (serde).

## Install

```bash
yarn add node-json-schema-transformer
```

Prebuilt binaries are published for macOS (x64/arm64), Linux (x64/arm64 gnu), and Windows (x64).

## Usage

### Convenience functions

One call per target language; the module string is returned:

```ts
import { jsonSchemaToZodModule } from 'node-json-schema-transformer'

const schema = {
  type: 'object',
  properties: {
    id: { type: 'string', format: 'uuid' },
    name: { type: 'string' },
  },
  required: ['id'],
}

const module = jsonSchemaToZodModule(schema, 'person')
// import { z } from "zod";
// export const PersonSchema = z.object({ id: z.string().uuid(), name: z.string().optional() });
// export type Person = z.infer<typeof PersonSchema>;
```

Also available: `jsonSchemaToTypescript`, `jsonSchemaToPydantic`, `jsonSchemaToSwift`, `jsonSchemaToKotlin`, `jsonSchemaToRust`.

The name argument is optional — when omitted, the schema's root `title` is used; if neither is present an error is thrown.

### `transform` with options

```ts
import { Language, transform } from 'node-json-schema-transformer'

const swift = transform(schema, Language.Swift, {
  name: 'person',
  // Swift `var` / Kotlin `var` / Pydantic assignment re-validation
  mutable: true,
  // treat `format` as annotation-only (draft 2020-12 behavior)
  enforceFormats: false,
  // resolve non-fragment $refs against remote documents
  remotes: { 'https://example.com/address.json': addressSchema },
})
```

### Collections

When generating many modules, use a `CollectionSession` so runtime helpers are shared via one companion file (tailored to exactly what the emitted modules need) instead of being inlined into every module:

```ts
import { CollectionSession, Language } from 'node-json-schema-transformer'

const session = new CollectionSession(Language.Zod)
const moduleA = session.emit(schemaA, 'EventA')
const moduleB = session.emit(schemaB, 'EventB', '../') // nested one directory down

const helpers = session.helpers()
if (helpers) {
  // write helpers.content to helpers.fileName at the collection root
}
```

### Language metadata

```ts
import { defaultHelpersFile, extensionFor, helpersContent, Language } from 'node-json-schema-transformer'

extensionFor(Language.Zod) // "zod.ts"
defaultHelpersFile(Language.Pydantic) // "jst_helpers.py"
helpersContent(Language.Kotlin) // full helpers file superset
```

## Develop

- Install the latest Rust toolchain, Node.js, [bun](https://bun.sh), and yarn 4 (via corepack)
- `yarn install`
- `yarn build:debug` — build the native addon and regenerate `index.js` / `index.d.ts`
- `yarn test` — run the test suite with `bun test`
- `yarn bench` — run benchmarks

## Release

Set **NPM_TOKEN** in the repository's GitHub secrets, then:

```bash
npm version [major | minor | patch | ...]
git push
```

GitHub Actions builds the per-platform binaries and publishes to npm. Don't run `npm publish` manually.
