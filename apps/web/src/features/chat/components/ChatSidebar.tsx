import {
  ChevronDown,
  ChevronLeft,
  Folder,
  LoaderCircle,
  LogOut,
  MessageSquarePlus,
  Search,
  X,
  Trash2,
} from 'lucide-react'
import { useMemo, useState } from 'react'
import clsx from 'clsx'
import type { SessionListItem } from '../useSessions'
import { groupSessionsByRelativeTime, SESSION_GROUP_PREVIEW_LIMIT } from '../sessionGrouping'

export function ChatSidebar({
  sessions,
  currentSessionId,
  loading,
  error,
  currentUserInitial,
  mobile = false,
  onSelect,
  onNewSession,
  onDeleteSession,
  onLogout,
  onClose,
}: {
  sessions: SessionListItem[]
  currentSessionId: string
  loading: boolean
  error: string | null
  currentUserInitial: string
  mobile?: boolean
  onSelect: (sessionId: string) => void
  onNewSession: () => void
  onDeleteSession: (sessionId: string) => void
  onLogout: () => void
  onClose?: () => void
}) {
  const [searchQuery, setSearchQuery] = useState('')
  const normalizedSearchQuery = searchQuery.trim().toLowerCase()
  const filteredSessions = useMemo(() => {
    if (!normalizedSearchQuery) {
      return sessions
    }

    return sessions.filter((session) => {
      const title = session.title?.trim() || ''
      const statusLabel = getSessionStatusLabel(session)
      return [
        title,
        session.id,
        session.status,
        statusLabel,
      ].some((value) => value.toLowerCase().includes(normalizedSearchQuery))
    })
  }, [normalizedSearchQuery, sessions])
  const groupedSessions = useMemo(
    () => groupSessionsByRelativeTime(filteredSessions),
    [filteredSessions],
  )
  const runningCount = useMemo(
    () => sessions.filter((session) => isRunningSession(session)).length,
    [sessions],
  )
  const isSearching = normalizedSearchQuery.length > 0

  return (
    <aside className="flex h-full w-[260px] max-w-[86vw] flex-shrink-0 flex-col border-r border-gray-100 bg-[#f8f8f8] dark:border-gray-800 dark:bg-[#121212]">
      <div className="flex min-h-0 flex-1 flex-col space-y-4 px-3 py-4">
        <div className="flex items-center justify-between rounded-lg px-2 py-1.5 transition-colors hover:bg-black/5 dark:hover:bg-white/5">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2.5">
              <div className="flex h-7 w-7 items-center justify-center rounded-md bg-black text-sm font-bold text-white dark:bg-white dark:text-black">
                {currentUserInitial}
              </div>
              <span className="text-[15px] font-semibold text-gray-900 dark:text-white">
                OpenChat
              </span>
            </div>
            <ChevronDown className="h-4 w-4 text-gray-400" />
          </div>
          {mobile ? (
            <button
              type="button"
              onClick={onClose}
              className="rounded-md p-1 text-gray-400 transition hover:bg-black/5 hover:text-gray-600 dark:hover:bg-white/5 dark:hover:text-gray-200"
              aria-label="Close sidebar"
            >
              <ChevronLeft className="h-4 w-4" />
            </button>
          ) : null}
        </div>

        <button
          type="button"
          onClick={() => {
            onNewSession()
            onClose?.()
          }}
          className="flex w-full items-center gap-2.5 rounded-md px-3 py-2 text-[14px] text-gray-600 transition-colors hover:bg-black/5 dark:text-gray-300 dark:hover:bg-white/5"
        >
          <MessageSquarePlus className="h-[18px] w-[18px] text-gray-500" />
          <span>开启新话题</span>
        </button>

        <div className="flex items-center gap-2.5 rounded-md px-3 py-2 text-[14px] text-gray-600 transition-colors hover:bg-black/5 focus-within:bg-black/5 dark:text-gray-300 dark:hover:bg-white/5 dark:focus-within:bg-white/5">
          <Search className="h-[18px] w-[18px] text-gray-400 flex-shrink-0" />
          <input
            type="text"
            placeholder="搜索"
            value={searchQuery}
            onChange={(event) => setSearchQuery(event.target.value)}
            className="w-full bg-transparent outline-none placeholder:text-gray-500 text-[14px]"
          />
          {searchQuery ? (
            <button
              type="button"
              onClick={() => setSearchQuery('')}
              className="-mr-1 rounded p-1 text-gray-400 transition hover:bg-black/10 hover:text-gray-700 dark:hover:bg-white/10 dark:hover:text-gray-200"
              aria-label="清空搜索"
            >
              <X className="h-3.5 w-3.5" />
            </button>
          ) : null}
        </div>

        <div className="-mx-1 flex-1 space-y-3 overflow-y-auto px-1">
          {runningCount > 0 && !isSearching ? (
            <div className="mx-1 flex items-center gap-2 rounded-md border border-blue-100 bg-blue-50 px-3 py-2 text-[12px] font-medium text-blue-700 dark:border-blue-900/50 dark:bg-blue-950/30 dark:text-blue-200">
              <LoaderCircle className="h-3.5 w-3.5 animate-spin" />
              <span>{runningCount} 个对话正在执行</span>
            </div>
          ) : null}

          {loading ? (
            <div className="flex items-center gap-2 p-2 text-sm text-gray-500">
              <LoaderCircle className="h-4 w-4 animate-spin" />
              <span>加载会话…</span>
            </div>
          ) : null}

          {error ? <p className="p-2 text-xs text-red-500">{error}</p> : null}

          {isSearching ? (
            <SearchResultsGroup
              sessions={filteredSessions}
              currentSessionId={currentSessionId}
              onSelect={onSelect}
              onDelete={onDeleteSession}
              onClose={onClose}
            />
          ) : null}

          {!isSearching && groupedSessions.today.length > 0 && (
            <SessionGroup
              title="今天"
              sessions={groupedSessions.today}
              currentSessionId={currentSessionId}
              onSelect={onSelect}
              onDelete={onDeleteSession}
            />
          )}

          {!isSearching && groupedSessions.thisWeek.length > 0 && (
            <SessionGroup
              title="本周"
              sessions={groupedSessions.thisWeek}
              currentSessionId={currentSessionId}
              onSelect={onSelect}
              onDelete={onDeleteSession}
            />
          )}

          {!isSearching && groupedSessions.older.length > 0 && (
            <SessionGroup
              title="更早"
              sessions={groupedSessions.older}
              currentSessionId={currentSessionId}
              onSelect={onSelect}
              onDelete={onDeleteSession}
            />
          )}

          {!loading && !error && sessions.length === 0 ? (
            <div className="px-3 py-8 text-center text-[13px] text-gray-400">暂无对话</div>
          ) : null}
        </div>
      </div>

      <div className="p-3">
        <button
          type="button"
          onClick={() => {
            onClose?.()
            onLogout()
          }}
          className="flex w-full items-center gap-2.5 rounded-md px-3 py-2 text-[14px] text-gray-600 transition-colors hover:bg-black/5 dark:text-gray-300 dark:hover:bg-white/5"
        >
          <LogOut className="h-[18px] w-[18px]" />
          <span>退出登录</span>
        </button>
      </div>
    </aside>
  )
}

const isRunningSession = (session: SessionListItem) => session.status === 'running'

const getSessionStatusLabel = (session: SessionListItem) => {
  if (isRunningSession(session)) return '生成中'
  if (session.status === 'failed') return '失败'
  if (session.status === 'interrupted') return '已中止'
  return ''
}

function SearchResultsGroup({
  sessions,
  currentSessionId,
  onSelect,
  onDelete,
  onClose,
}: {
  sessions: SessionListItem[]
  currentSessionId: string
  onSelect: (id: string) => void
  onDelete: (id: string) => void
  onClose?: () => void
}) {
  return (
    <div>
      <div className="px-3 py-1.5 text-[12px] font-medium text-gray-400">
        搜索结果 {sessions.length} 条
      </div>
      {sessions.length > 0 ? (
        <div className="space-y-0.5">
          {sessions.map((session) => (
            <SessionRow
              key={session.id}
              session={session}
              currentSessionId={currentSessionId}
              onSelect={(id) => {
                onSelect(id)
                onClose?.()
              }}
              onDelete={onDelete}
            />
          ))}
        </div>
      ) : (
        <div className="px-3 py-8 text-center text-[13px] text-gray-400">
          没有匹配的对话
        </div>
      )}
    </div>
  )
}

function SessionGroup({
  title,
  sessions,
  currentSessionId,
  onSelect,
  onDelete,
}: {
  title: string
  sessions: SessionListItem[]
  currentSessionId: string
  onSelect: (id: string) => void
  onDelete: (id: string) => void
}) {
  const [showAll, setShowAll] = useState(false)
  const limit = SESSION_GROUP_PREVIEW_LIMIT
  const displayedSessions = showAll ? sessions : sessions.slice(0, limit)
  const hiddenCount = sessions.length - limit
  const hasHiddenSessions = hiddenCount > 0

  return (
    <div>
      <div className="flex items-center gap-1 px-3 py-1.5 text-[12px] font-medium text-gray-400">
        <span>{title}</span>
        <ChevronDown
          className={clsx(
            'h-3 w-3 transition-transform',
            showAll ? 'rotate-180' : 'rotate-0',
            hasHiddenSessions ? 'opacity-100' : 'opacity-0',
          )}
        />
      </div>
      <div className="space-y-0.5">
        {displayedSessions.map((session) => (
          <SessionRow
            key={session.id}
            session={session}
            currentSessionId={currentSessionId}
            onSelect={onSelect}
            onDelete={onDelete}
          />
        ))}
        {hasHiddenSessions && (
          <button
            type="button"
            onClick={() => setShowAll(!showAll)}
            className="w-full px-3 py-1.5 text-left text-[13px] text-gray-400 transition-colors hover:text-gray-600"
            aria-expanded={showAll}
          >
            {showAll ? '收起' : `展开显示 ${hiddenCount} 条`}
          </button>
        )}
      </div>
    </div>
  )
}

function SessionRow({
  session,
  currentSessionId,
  onSelect,
  onDelete,
}: {
  session: SessionListItem
  currentSessionId: string
  onSelect: (id: string) => void
  onDelete: (id: string) => void
}) {
  const active = session.id === currentSessionId
  const running = isRunningSession(session)
  const statusLabel = getSessionStatusLabel(session)

  return (
    <div
      className={clsx(
        'group flex cursor-pointer items-center justify-between rounded-md px-3 py-2 text-[14px] transition-colors',
        active
          ? 'bg-[#e5f0ff] text-blue-600 dark:bg-blue-900/30 dark:text-blue-400'
          : 'text-gray-700 hover:bg-black/5 dark:text-gray-300 dark:hover:bg-white/5',
      )}
      onClick={() => {
        onSelect(session.id)
      }}
    >
      <div className="flex min-w-0 items-center gap-2.5">
        {running ? (
          <LoaderCircle
            className={clsx(
              'h-[16px] w-[16px] flex-shrink-0 animate-spin',
              active ? 'text-blue-500' : 'text-blue-500 dark:text-blue-300',
            )}
          />
        ) : (
          <Folder
            className={clsx(
              'h-[16px] w-[16px] flex-shrink-0',
              active ? 'text-blue-500' : 'text-gray-400',
            )}
          />
        )}
        <div className="min-w-0">
          <div className="truncate">{session.title?.trim() || session.id}</div>
          {statusLabel ? (
            <div
              className={clsx(
                'mt-0.5 text-[11px] leading-none',
                running ? 'text-blue-500 dark:text-blue-300' : 'text-gray-400',
              )}
            >
              {statusLabel}
            </div>
          ) : null}
        </div>
      </div>
      <button
        type="button"
        className="rounded p-1 text-gray-400 opacity-0 transition-all hover:bg-black/10 hover:text-red-500 group-hover:opacity-100 dark:hover:bg-white/10"
        onClick={(event) => {
          event.stopPropagation()
          onDelete(session.id)
        }}
        aria-label="Delete session"
      >
        <Trash2 className="h-3.5 w-3.5" />
      </button>
    </div>
  )
}
