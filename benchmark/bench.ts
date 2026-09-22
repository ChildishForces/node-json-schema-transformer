import { Bench } from 'tinybench'

import { jsonSchemaToZodModule, Language, transform } from '../index.js'

const schema = {
  type: 'object',
  properties: {
    id: { type: 'string', format: 'uuid' },
    name: { type: 'string', minLength: 1, maxLength: 50 },
    email: { type: 'string', format: 'email' },
    tags: { type: 'array', items: { type: 'string' }, uniqueItems: true },
    age: { type: 'integer', minimum: 0 },
  },
  required: ['id', 'name'],
}

const b = new Bench()

b.add('jsonSchemaToZodModule', () => {
  jsonSchemaToZodModule(schema, 'person')
})

b.add('transform → typescript', () => {
  transform(schema, Language.TypeScript, { name: 'person' })
})

b.add('transform → rust', () => {
  transform(schema, Language.Rust, { name: 'person' })
})

await b.run()

console.table(b.table())
