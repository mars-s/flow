import { createFileRoute } from '@tanstack/react-router'

export const Route = createFileRoute('/')({
  component: Home,
})

function Home() {
  return (
    <div className="flex min-h-dvh flex-col items-center justify-center gap-3 px-6 text-center antialiased">
      <img src="/app-icon.png" alt="" className="size-12 rounded-[10px]" />
      <h1 className="text-2xl font-semibold tracking-tight">Flow</h1>
      <p className="max-w-md text-sm leading-relaxed text-muted-foreground">
        A calm, keyboard-first personal task manager with a read-only
        calendar glance. This site is a placeholder — there is nothing to
        download yet.
      </p>
    </div>
  )
}
