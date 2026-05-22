import type { SessionListItem } from './useSessions'
import { parseTimestamp, shanghaiDayIndex, shanghaiWeekStartDayIndex } from './timestamps'

export const SESSION_GROUP_PREVIEW_LIMIT = 3

export interface SessionGroup {
  key: string
  title: string
  sessions: SessionListItem[]
}

const createEmptyGroups = (): SessionGroup[] => [
  { key: 'today', title: '今天', sessions: [] },
  { key: 'yesterday', title: '昨天', sessions: [] },
  { key: 'beforeYesterday', title: '前天', sessions: [] },
  { key: 'thisWeek', title: '本周', sessions: [] },
  { key: 'lastWeek', title: '上周', sessions: [] },
  { key: 'thisMonth', title: '本月', sessions: [] },
  { key: 'older', title: '更早', sessions: [] },
]

export const groupSessionsByRelativeTime = (sessions: SessionListItem[]): SessionGroup[] => {
  const nowTimestamp = Date.now()
  const todayIndex = shanghaiDayIndex(nowTimestamp)
  const yesterdayIndex = todayIndex - 1
  const beforeYesterdayIndex = todayIndex - 2
  const weekStartIndex = shanghaiWeekStartDayIndex(nowTimestamp)
  const lastWeekStartIndex = weekStartIndex - 7
  const shanghaiNow = new Date(nowTimestamp + 8 * 60 * 60 * 1000)
  const monthStartIndex = shanghaiDayIndex(
    Date.UTC(shanghaiNow.getUTCFullYear(), shanghaiNow.getUTCMonth(), 1) - 8 * 60 * 60 * 1000,
  )
  const groups = createEmptyGroups()
  const groupByKey = new Map(groups.map((group) => [group.key, group]))

  for (const session of sessions) {
    const sessionDayIndex = shanghaiDayIndex(parseTimestamp(session.updatedAt))
    let groupKey = 'older'

    if (sessionDayIndex === todayIndex) {
      groupKey = 'today'
    } else if (sessionDayIndex === yesterdayIndex) {
      groupKey = 'yesterday'
    } else if (sessionDayIndex === beforeYesterdayIndex) {
      groupKey = 'beforeYesterday'
    } else if (sessionDayIndex >= weekStartIndex) {
      groupKey = 'thisWeek'
    } else if (sessionDayIndex >= lastWeekStartIndex) {
      groupKey = 'lastWeek'
    } else if (sessionDayIndex >= monthStartIndex) {
      groupKey = 'thisMonth'
    }

    groupByKey.get(groupKey)?.sessions.push(session)
  }

  return groups.filter((group) => group.sessions.length > 0)
}
