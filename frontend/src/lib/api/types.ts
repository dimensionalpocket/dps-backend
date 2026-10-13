export interface Role {
  id: number
  name: string
  permissions: string[]
}

export interface User {
  id: number
  uuid?: string
  name: string
  role: Role
  metadataJson?: string | null
  createdTs?: number
  updatedTs?: number
}

export interface AuthMe {
  user: User
  sessionIat: number
  sessionExp: number
}

export interface AuthLogin {
  token: string
  user: User
  message: string
}

export interface AuthRegister {
  user: User
  message: string
}

export interface ChangePasswordResult {
  message: string
  updatedTs: number
}

export interface Site {
  id: number
  slug: string
  subdomain: string | null
  port: number | null
  protocol: string
  metadataJson?: string | null
}
