import { toast } from 'vue-sonner'

export interface ToastOptions {
  duration?: number
  position?: 'top-left' | 'top-center' | 'top-right' | 'bottom-left' | 'bottom-center' | 'bottom-right'
  action?: {
    label: string
    onClick: () => void
  }
}

export const useToast = () => {
  return {
    success: (message: string, options?: ToastOptions) => {
      return toast.success(message, {
        duration: options?.duration || 5000,
        position: options?.position || 'bottom-center',
        ...options
      })
    },
    error: (message: string, options?: ToastOptions) => {
      return toast.error(message, {
        duration: options?.duration || 8000,
        position: options?.position || 'bottom-center',
        ...options
      })
    },
    info: (message: string, options?: ToastOptions) => {
      return toast.info(message, {
        duration: options?.duration || 5000,
        position: options?.position || 'bottom-center',
        ...options
      })
    },
    warning: (message: string, options?: ToastOptions) => {
      return toast.warning(message, {
        duration: options?.duration || 8000,
        position: options?.position || 'bottom-center',
        ...options
      })
    },
    loading: (message: string, options?: ToastOptions) => {
      return toast.loading(message, {
        position: options?.position || 'bottom-center',
        ...options
      })
    },
    dismiss: (id?: string | number) => {
      toast.dismiss(id)
    }
  }
}