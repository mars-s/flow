# flow-core

Generic, reusable infrastructure kept when Flow was stripped down from
Flow's coding-agent desktop app. Everything agent/daemon-specific (session
drivers, persistence, Git and workspace automation, the daemon RPC server,
usage tracking, and so on) was deleted; only locale plumbing
(`i18n`) and application identity (`identity`) survived as genuinely generic.
