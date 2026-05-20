import { Navigate, Route, Routes } from 'react-router-dom'
import { LoginView } from './features/auth/components/LoginView'
import { RequireAuth, RequireGuest } from './features/auth/components/AuthRouteGuards'
import { useAuthSession } from './features/auth/useAuthSession'
import {
  CHAT_HOME_PATH,
  CHAT_INDEX_PATH,
  CHAT_SESSION_PATH,
} from './features/chat/chatNavigation'
import { ChatWorkspaceRoute } from './features/chat/components/ChatWorkspaceRoute'

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

  return (
    <Routes>
      <Route
        path="/login"
        element={
          <RequireGuest currentUser={currentUser}>
            <LoginView
              mode="login"
              loading={authSubmitting}
              error={authError}
              onLogin={handleLogin}
              onRegister={handleRegister}
            />
          </RequireGuest>
        }
      />
      <Route
        path="/register"
        element={
          <RequireGuest currentUser={currentUser}>
            <LoginView
              mode="register"
              loading={authSubmitting}
              error={authError}
              onLogin={handleLogin}
              onRegister={handleRegister}
            />
          </RequireGuest>
        }
      />
      <Route
        path={CHAT_HOME_PATH}
        element={
          <RequireAuth currentUser={currentUser}>
            <ChatWorkspaceRoute
              currentUser={currentUser!}
              onLogout={handleLogout}
              onUnauthorized={handleUnauthorized}
            />
          </RequireAuth>
        }
      />
      <Route
        path={CHAT_INDEX_PATH}
        element={
          <RequireAuth currentUser={currentUser}>
            <ChatWorkspaceRoute
              currentUser={currentUser!}
              onLogout={handleLogout}
              onUnauthorized={handleUnauthorized}
            />
          </RequireAuth>
        }
      />
      <Route
        path={CHAT_SESSION_PATH}
        element={
          <RequireAuth currentUser={currentUser}>
            <ChatWorkspaceRoute
              currentUser={currentUser!}
              onLogout={handleLogout}
              onUnauthorized={handleUnauthorized}
            />
          </RequireAuth>
        }
      />
      <Route path="*" element={<Navigate to={currentUser ? '/' : '/login'} replace />} />
    </Routes>
  )
}
