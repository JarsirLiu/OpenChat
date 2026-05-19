import type { SessionListItem } from './useSessions'

export const SESSION_GROUP_PREVIEW_LIMIT = 3

export type SessionGroupKey = 'today' | 'thisWeek' | 'older'

export type SessionGroups = Record<SessionGroupKey, SessionListItem[]>

const parseSessionTime = (value: string) => {
  const timestamp = Number(value)
  return Number.isFinite(timestamp) ? timestamp : 0
}

const formatLocalDateKey = (date: Date) => {
  const year = date.getFullYear()
  const month = String(date.getMonth() + 1).padStart(2, '0')
  const day = String(date.getDate()).padStart(2, '0')
  return `${year}-${month}-${day}`
}

const startOfLocalWeek = (date: Date) => {
  const result = new Date(date)
  const dayIndex = result.getDay()
  const offset = (dayIndex + 6) % 7
  result.setDate(result.getDate() - offset)
  result.setHours(0, 0, 0, 0)
  return result
}

export const groupSessionsByRelativeTime = (sessions: SessionListItem[]): SessionGroups => {
  const now = new Date()
  const todayKey = formatLocalDateKey(now)
  const weekStart = startOfLocalWeek(now)

  const sortedSessions = [...sessions].sort(
    (left, right) =>
      parseSessionTime(right.updatedAt) - parseSessionTime(left.updatedAt) ||
      parseSessionTime(right.createdAt) - parseSessionTime(left.createdAt),
  )

  return sortedSessions.reduce<SessionGroups>(
    (acc, session) => {
      const sessionTime = new Date(parseSessionTime(session.updatedAt))

      if (formatLocalDateKey(sessionTime) === todayKey) {
        acc.today.push(session)
      } else if (sessionTime >= weekStart) {
        acc.thisWeek.push(session)
      } else {
        acc.older.push(session)
      }

      return acc
    },
    {
      today: [],
      thisWeek: [],
      older: [],
    },
  )
}
