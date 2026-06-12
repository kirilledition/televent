import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const scriptDir = path.dirname(fileURLToPath(import.meta.url))
const repoRoot = path.resolve(scriptDir, '..', '..')
const openApiPath = path.join(repoRoot, 'backend', 'docs', 'openapi.json')
const outputPath = path.join(repoRoot, 'frontend', 'src', 'types', 'schema.ts')

const openapi = JSON.parse(fs.readFileSync(openApiPath, 'utf8'))
const schemas = openapi.components?.schemas ?? {}

const schemaOrder = [
  'CreateDeviceRequest',
  'EventTimingRequest',
  'CreateEventRequest',
  'DeviceListItem',
  'DevicePasswordResponse',
  'EventStatus',
  'EventResponse',
  'ListEventsQuery',
  'UpdateEventRequest',
  'MeResponse',
  'CalendarInfo',
]

function refName(ref) {
  return ref.split('/').at(-1)
}

function enumMemberName(value) {
  return value
    .replace(/[^A-Za-z0-9]+/g, ' ')
    .trim()
    .split(/\s+/)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join('')
}

function quoteTs(value) {
  return `'${String(value).replace(/\\/g, '\\\\').replace(/'/g, "\\'")}'`
}

function renderType(schema, context = {}) {
  if (!schema) {
    return 'unknown'
  }

  if (schema.$ref) {
    return refName(schema.$ref)
  }

  if (schema.oneOf) {
    return schema.oneOf.map((item) => renderType(item, context)).join(' | ')
  }

  if (Array.isArray(schema.type)) {
    const types = schema.type.map((type) =>
      type === 'null' ? 'null' : renderType({ ...schema, type }, context)
    )
    return [...new Set(types)].join(' | ')
  }

  if (schema.type === 'null') {
    return 'null'
  }

  if (schema.type === 'array') {
    return `${renderType(schema.items, context)}[]`
  }

  if (schema.enum) {
    return schema.enum.map((value) => quoteTs(value)).join(' | ')
  }

  if (schema.type === 'integer' || schema.type === 'number') {
    return 'number'
  }

  if (schema.type === 'boolean') {
    return 'boolean'
  }

  if (schema.type === 'object') {
    return renderInlineObject(schema, context)
  }

  if (schema.type === 'string') {
    if (context.propertyName === 'timezone') {
      return 'Timezone'
    }
    if (
      context.propertyName === 'id' &&
      (context.schemaName === 'MeResponse' ||
        context.schemaName === 'CalendarInfo')
    ) {
      return 'UserId'
    }
    return 'string'
  }

  return 'unknown'
}

function renderInlineObject(schema, context = {}) {
  const required = new Set(schema.required ?? [])
  const properties = schema.properties ?? {}
  const lines = Object.entries(properties).map(([propertyName, property]) => {
    const optional = required.has(propertyName) ? '' : '?'
    return `  ${propertyName}${optional}: ${renderType(property, {
      ...context,
      propertyName,
    })}`
  })

  if (lines.length === 0) {
    return 'Record<string, unknown>'
  }

  return `{\n${lines.join('\n')}\n}`
}

function renderObjectInterface(name, schema) {
  const required = new Set(schema.required ?? [])
  const properties = schema.properties ?? {}
  const lines = [`export interface ${name} {`]

  for (const [propertyName, property] of Object.entries(properties)) {
    const optional = required.has(propertyName) ? '' : '?'
    lines.push(
      `  ${propertyName}${optional}: ${renderType(property, {
        schemaName: name,
        propertyName,
      })}`
    )
  }

  lines.push('}')
  return lines.join('\n')
}

function renderEnum(name, schema) {
  const lines = [`export enum ${name} {`]

  for (const value of schema.enum) {
    lines.push(`  ${enumMemberName(value)} = ${quoteTs(value)},`)
  }

  lines.push('}')
  return lines.join('\n')
}

function renderOneOf(name, schema) {
  const variants = schema.oneOf.map((variant) =>
    renderInlineObject(variant, { schemaName: name })
  )
  return `export type ${name} =\n${variants.map((variant) => `  | ${indentMultiline(variant, 4).trimStart()}`).join('\n')}`
}

function indentMultiline(value, spaces) {
  const prefix = ' '.repeat(spaces)
  return value
    .split('\n')
    .map((line) => `${prefix}${line}`)
    .join('\n')
}

function renderSchema(name, schema) {
  if (schema.type === 'string' && schema.enum) {
    return renderEnum(name, schema)
  }

  if (schema.oneOf) {
    return renderOneOf(name, schema)
  }

  if (schema.type === 'object') {
    return renderObjectInterface(name, schema)
  }

  return `export type ${name} = ${renderType(schema, { schemaName: name })}`
}

const sections = [
  '// Generated from backend/docs/openapi.json.',
  '// Run `just gen-types` to refresh.',
  'export type Timezone = string',
  'export type UserId = string',
  ...schemaOrder
    .filter((name) => schemas[name])
    .map((name) => renderSchema(name, schemas[name])),
  'export type Event = EventResponse',
]

const generated = `${sections.join('\n\n')}\n`

fs.writeFileSync(outputPath, generated)
console.log(
  `Generated ${path.relative(repoRoot, outputPath)} from ${path.relative(repoRoot, openApiPath)}`
)
