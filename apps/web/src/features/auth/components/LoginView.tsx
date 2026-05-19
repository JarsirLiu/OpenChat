import { useState } from 'react'

interface LoginViewProps {
  loading: boolean
  error: string | null
  onLogin: (account: string, password: string) => Promise<void>
  onRegister: (email: string, password: string, username?: string) => Promise<void>
}

export function LoginView({ loading, error, onLogin, onRegister }: LoginViewProps) {
  const [mode, setMode] = useState<'login' | 'register'>('login')
  const [account, setAccount] = useState('')
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')

  const isRegister = mode === 'register'

  return (
    <div className="flex min-h-screen items-center justify-center bg-[#f7f7f8] px-4">
      <div className="w-full max-w-[420px] rounded-3xl border border-gray-200 bg-white p-8 shadow-[0_20px_60px_rgba(0,0,0,0.08)]">
        <div className="mb-4 inline-flex rounded-full bg-gray-900 px-3 py-1.5 text-sm font-bold text-white">
          OpenChat
        </div>
        <h1 className="text-[28px] text-gray-900">
          {isRegister ? '邮箱注册' : '邮箱账号登录'}
        </h1>
        <p className="mt-2 text-sm leading-6 text-gray-500">
          {isRegister
            ? '创建 OpenChat 账号后，就可以进入聊天页、选择文本模型，并按需启用生图模型。'
            : '使用 OpenChat 账号登录后，再进入聊天页与模型目录。'}
        </p>

        <div className="mt-6 grid grid-cols-2 gap-1.5 rounded-2xl bg-gray-100 p-1.5">
          <button
            type="button"
            className={`h-10 rounded-xl text-sm ${
              !isRegister ? 'bg-white text-gray-900 shadow-sm' : 'text-gray-500'
            }`}
            onClick={() => setMode('login')}
          >
            登录
          </button>
          <button
            type="button"
            className={`h-10 rounded-xl text-sm ${
              isRegister ? 'bg-white text-gray-900 shadow-sm' : 'text-gray-500'
            }`}
            onClick={() => setMode('register')}
          >
            注册
          </button>
        </div>

        <form
          className="mt-6 grid gap-3.5"
          onSubmit={async (event) => {
            event.preventDefault()
            if (isRegister) {
              await onRegister(account, password, username)
              return
            }
            await onLogin(account, password)
          }}
        >
          {isRegister ? (
            <label className="grid gap-2 text-sm text-gray-700">
              <span>用户名</span>
              <input
                className="w-full rounded-2xl border border-gray-300 px-3.5 py-3 outline-none"
                value={username}
                autoComplete="username"
                onChange={(event) => setUsername(event.target.value)}
              />
            </label>
          ) : null}

          <label className="grid gap-2 text-sm text-gray-700">
            <span>邮箱</span>
            <input
              className="w-full rounded-2xl border border-gray-300 px-3.5 py-3 outline-none"
              value={account}
              type="email"
              autoComplete={isRegister ? 'email' : 'username'}
              onChange={(event) => setAccount(event.target.value)}
            />
          </label>

          <label className="grid gap-2 text-sm text-gray-700">
            <span>密码</span>
            <input
              className="w-full rounded-2xl border border-gray-300 px-3.5 py-3 outline-none"
              type="password"
              value={password}
              autoComplete={isRegister ? 'new-password' : 'current-password'}
              onChange={(event) => setPassword(event.target.value)}
            />
          </label>

          {isRegister && password.length > 0 && password.length < 6 ? (
            <p className="rounded-xl border border-amber-300 bg-amber-50 px-3 py-2 text-sm text-amber-700">
              注册密码至少需要 6 位。
            </p>
          ) : null}

          {error ? (
            <p className="rounded-xl border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-700">
              {error}
            </p>
          ) : null}

          <button
            type="submit"
            className="mt-1 inline-flex h-11 items-center justify-center rounded-2xl bg-gray-900 text-white"
            disabled={loading || !account.trim() || !password || (isRegister && password.length < 6)}
          >
            {loading ? (isRegister ? '注册中…' : '登录中…') : isRegister ? '注册' : '登录'}
          </button>
        </form>
      </div>
    </div>
  )
}
