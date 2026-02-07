export {}

declare global {
  type Uuid = string
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  type DateTime<_T> = string
  type Utc = unknown
}
