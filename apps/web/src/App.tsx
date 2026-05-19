import { LoginView } from './features/auth/components/LoginView'
import { useAuthSession } from './features/auth/useAuthSession'
import { ChatWorkspace } from './features/chat/components/ChatWorkspace'

export function App() {
  const {
    authLoading,
    authSubmitting,
    authError,
    currentUser,
    handleLogin,
    handleLogout,
    handleRegister,
    handleUnauthorized,
  } = useAuthSession()

  if (authLoading) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-[#f7f7f8] text-gray-600">
        Checking session…
      </div>
    )
  }

  if (!currentUser) {
    return (
      <LoginView
        loading={authSubmitting}
        error={authError}
        onLogin={handleLogin}
        onRegister={handleRegister}
      />
    )
  }

  return (
    <ChatWorkspace
      currentUser={currentUser}
      onLogout={handleLogout}
      onUnauthorized={handleUnauthorized}
    />
  )
}
