const SHANGHAI_TIME_ZONE = 'Asia/Shanghai'
const SHANGHAI_OFFSET = '+08:00'
const SHANGHAI_OFFSET_MS = 8 * 60 * 60 * 1000
const DAY_MS = 24 * 60 * 60 * 1000

export const parseTimestamp = (value: string) => {
  const numeric = Number(value)
  if (Number.isFinite(numeric)) {
    return numeric
  }

  const parsed = Date.parse(value)
  return Number.isNaN(parsed) ? 0 : parsed
}

export const createShanghaiTimestamp = () => {
  const formatter = new Intl.DateTimeFormat('sv-SE', {
    timeZone: SHANGHAI_TIME_ZONE,
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false,
  })

  const parts = Object.fromEntries(
    formatter
      .formatToParts(new Date())
      .filter((part) => part.type !== 'literal')
      .map((part) => [part.type, part.value]),
  )

  return `${parts.year}-${parts.month}-${parts.day}T${parts.hour}:${parts.minute}:${parts.second}.000${SHANGHAI_OFFSET}`
}

export const shanghaiDayIndex = (timestamp: number) =>
  Math.floor((timestamp + SHANGHAI_OFFSET_MS) / DAY_MS)

export const shanghaiWeekStartDayIndex = (timestamp: number) => {
  const shanghaiDate = new Date(timestamp + SHANGHAI_OFFSET_MS)
  const weekday = (shanghaiDate.getUTCDay() + 6) % 7
  return shanghaiDayIndex(timestamp) - weekday
}
