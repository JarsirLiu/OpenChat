import type { SessionListItem } from './useSessions'
import { parseTimestamp, shanghaiDayIndex, shanghaiWeekStartDayIndex } from './timestamps'

export const SESSION_GROUP_PREVIEW_LIMIT = 3

export type SessionGroupKey = 'today' | 'thisWeek' | 'older'

export type SessionGroups = Record<SessionGroupKey, SessionListItem[]>

export const groupSessionsByRelativeTime = (sessions: SessionListItem[]): SessionGroups => {
  const nowTimestamp = Date.now()
  const todayIndex = shanghaiDayIndex(nowTimestamp)
  const weekStartIndex = shanghaiWeekStartDayIndex(nowTimestamp)

  const sortedSessions = [...sessions].sort(
    (left, right) =>
      parseTimestamp(right.updatedAt) - parseTimestamp(left.updatedAt) ||
      parseTimestamp(right.createdAt) - parseTimestamp(left.createdAt),
  )

  return sortedSessions.reduce<SessionGroups>(
    (acc, session) => {
      const sessionDayIndex = shanghaiDayIndex(parseTimestamp(session.updatedAt))

      if (sessionDayIndex === todayIndex) {
        acc.today.push(session)
      } else if (sessionDayIndex >= weekStartIndex) {
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
