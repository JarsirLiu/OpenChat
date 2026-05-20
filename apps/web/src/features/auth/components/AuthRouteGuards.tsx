import type { PropsWithChildren } from 'react'
import { Navigate, useLocation } from 'react-router-dom'
import type { AuthUser } from '../../../lib/auth'

interface AuthGuardProps extends PropsWithChildren {
  currentUser: AuthUser | null
}

export function RequireAuth({ currentUser, children }: AuthGuardProps) {
  const location = useLocation()

  if (!currentUser) {
    return <Navigate to="/login" replace state={{ from: location }} />
  }

  return <>{children}</>
}

export function RequireGuest({ currentUser, children }: AuthGuardProps) {
  const location = useLocation()
  const redirectTo = location.state?.from?.pathname ?? '/'

  if (currentUser) {
    return <Navigate to={redirectTo} replace />
  }

  return <>{children}</>
}
