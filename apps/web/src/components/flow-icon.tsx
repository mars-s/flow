import type { ProviderKind } from '@flow/client'

export const FLOW_ICONS = {
  alert: 'i-flow-alert',
  appearance: 'i-flow-appearance',
  arrowDown: 'i-flow-arrow-down',
  arrowLeft: 'i-flow-arrow-left',
  arrowRight: 'i-flow-arrow-right',
  arrowUp: 'i-flow-arrow-up',
  arrowUpRight: 'i-flow-arrow-up-right',
  bot: 'i-flow-bot',
  chartColumn: 'i-flow-chart-column',
  check: 'i-flow-check',
  chevronDown: 'i-flow-chevron-down',
  chevronRight: 'i-flow-chevron-right',
  cloudUpload: 'i-flow-cloud-upload',
  command: 'i-flow-command',
  compose: 'i-flow-compose',
  copy: 'i-flow-copy',
  cornerDownRight: 'i-flow-corner-down-right',
  ellipsis: 'i-flow-ellipsis',
  eye: 'i-flow-eye',
  eyeOff: 'i-flow-eye-off',
  file: 'i-flow-file',
  fileDiff: 'i-flow-file-diff',
  folder: 'i-flow-folder',
  folderNew: 'i-flow-folder-new',
  fork: 'i-flow-fork',
  gauge: 'i-flow-gauge',
  gitBranch: 'i-flow-git-branch',
  gitCommitHorizontal: 'i-flow-git-commit-horizontal',
  globe: 'i-flow-globe',
  github: 'i-flow-github',
  info: 'i-flow-info',
  laptop: 'i-flow-laptop',
  list: 'i-flow-list',
  loaderCircle: 'i-flow-loader-circle',
  lock: 'i-flow-lock',
  lockOpen: 'i-flow-lock-open',
  package: 'i-flow-package',
  paperclip: 'i-flow-paperclip',
  panelLeft: 'i-flow-panel-left',
  panelRight: 'i-flow-panel-right',
  pencil: 'i-flow-pencil',
  plus: 'i-flow-plus',
  queue: 'i-flow-queue',
  rotateCw: 'i-flow-rotate-cw',
  rewind: 'i-flow-rewind',
  search: 'i-flow-search',
  server: 'i-flow-server',
  settings: 'i-flow-settings',
  sparkle: 'i-flow-sparkle',
  star: 'i-flow-star',
  starFilled: 'i-flow-star-filled',
  stop: 'i-flow-stop',
  stopFilled: 'i-flow-stop-filled',
  terminal: 'i-flow-terminal',
  terminalSquare: 'i-flow-terminal-square',
  trash: 'i-flow-trash',
  wrench: 'i-flow-wrench',
  x: 'i-flow-x',
  zap: 'i-flow-zap',
} as const

export type FlowIconName = keyof typeof FLOW_ICONS

export function FlowIcon({
  name,
  className,
  label,
}: {
  name: FlowIconName
  className?: string
  label?: string
}) {
  return (
    <span
      aria-hidden={label ? undefined : true}
      aria-label={label}
      className={`inline-grid size-4 shrink-0 place-items-center ${className ?? ''}`}
      role={label ? 'img' : undefined}
    >
      <span
        aria-hidden="true"
        className={FLOW_ICONS[name]}
        style={{ width: '100%', height: '100%' }}
      />
    </span>
  )
}

const FILE_TYPE_ICONS = {
  angular: 'i-flow-file-type-angular',
  astro: 'i-flow-file-type-astro',
  audio: 'i-flow-file-type-audio',
  babel: 'i-flow-file-type-babel',
  biome: 'i-flow-file-type-biome',
  bun: 'i-flow-file-type-bun',
  c: 'i-flow-file-type-c',
  certificate: 'i-flow-file-type-certificate',
  clojure: 'i-flow-file-type-clojure',
  cmake: 'i-flow-file-type-cmake',
  coffee: 'i-flow-file-type-coffee',
  console: 'i-flow-file-type-console',
  cpp: 'i-flow-file-type-cpp',
  crystal: 'i-flow-file-type-crystal',
  csharp: 'i-flow-file-type-csharp',
  css: 'i-flow-file-type-css',
  dart: 'i-flow-file-type-dart',
  database: 'i-flow-file-type-database',
  deno: 'i-flow-file-type-deno',
  diff: 'i-flow-file-type-diff',
  docker: 'i-flow-file-type-docker',
  editorconfig: 'i-flow-file-type-editorconfig',
  elixir: 'i-flow-file-type-elixir',
  elm: 'i-flow-file-type-elm',
  erlang: 'i-flow-file-type-erlang',
  eslint: 'i-flow-file-type-eslint',
  exe: 'i-flow-file-type-exe',
  file: 'i-flow-file-type-file',
  firebase: 'i-flow-file-type-firebase',
  git: 'i-flow-file-type-git',
  gitlab: 'i-flow-file-type-gitlab',
  go: 'i-flow-file-type-go',
  gradle: 'i-flow-file-type-gradle',
  graphql: 'i-flow-file-type-graphql',
  haskell: 'i-flow-file-type-haskell',
  haxe: 'i-flow-file-type-haxe',
  helm: 'i-flow-file-type-helm',
  html: 'i-flow-file-type-html',
  image: 'i-flow-file-type-image',
  java: 'i-flow-file-type-java',
  javascript: 'i-flow-file-type-javascript',
  jinja: 'i-flow-file-type-jinja',
  json: 'i-flow-file-type-json',
  julia: 'i-flow-file-type-julia',
  kotlin: 'i-flow-file-type-kotlin',
  kubernetes: 'i-flow-file-type-kubernetes',
  lock: 'i-flow-file-type-lock',
  lua: 'i-flow-file-type-lua',
  makefile: 'i-flow-file-type-makefile',
  markdown: 'i-flow-file-type-markdown',
  nest: 'i-flow-file-type-nest',
  next: 'i-flow-file-type-next',
  nginx: 'i-flow-file-type-nginx',
  nix: 'i-flow-file-type-nix',
  nodejs: 'i-flow-file-type-nodejs',
  npm: 'i-flow-file-type-npm',
  nuxt: 'i-flow-file-type-nuxt',
  ocaml: 'i-flow-file-type-ocaml',
  pdf: 'i-flow-file-type-pdf',
  perl: 'i-flow-file-type-perl',
  php: 'i-flow-file-type-php',
  pnpm: 'i-flow-file-type-pnpm',
  powershell: 'i-flow-file-type-powershell',
  prettier: 'i-flow-file-type-prettier',
  prisma: 'i-flow-file-type-prisma',
  proto: 'i-flow-file-type-proto',
  pug: 'i-flow-file-type-pug',
  python: 'i-flow-file-type-python',
  react: 'i-flow-file-type-react',
  readme: 'i-flow-file-type-readme',
  rollup: 'i-flow-file-type-rollup',
  ruby: 'i-flow-file-type-ruby',
  rust: 'i-flow-file-type-rust',
  sass: 'i-flow-file-type-sass',
  scala: 'i-flow-file-type-scala',
  settings: 'i-flow-file-type-settings',
  solidity: 'i-flow-file-type-solidity',
  storybook: 'i-flow-file-type-storybook',
  stylelint: 'i-flow-file-type-stylelint',
  supabase: 'i-flow-file-type-supabase',
  svelte: 'i-flow-file-type-svelte',
  svg: 'i-flow-file-type-svg',
  swift: 'i-flow-file-type-swift',
  tailwindcss: 'i-flow-file-type-tailwindcss',
  terraform: 'i-flow-file-type-terraform',
  tex: 'i-flow-file-type-tex',
  turborepo: 'i-flow-file-type-turborepo',
  typescript: 'i-flow-file-type-typescript',
  video: 'i-flow-file-type-video',
  vite: 'i-flow-file-type-vite',
  vitest: 'i-flow-file-type-vitest',
  vue: 'i-flow-file-type-vue',
  webassembly: 'i-flow-file-type-webassembly',
  webpack: 'i-flow-file-type-webpack',
  xaml: 'i-flow-file-type-xaml',
  xml: 'i-flow-file-type-xml',
  yaml: 'i-flow-file-type-yaml',
  yarn: 'i-flow-file-type-yarn',
  zig: 'i-flow-file-type-zig',
  zip: 'i-flow-file-type-zip',
} as const

type FileTypeIconName = keyof typeof FILE_TYPE_ICONS

export function FileTypeIcon({
  path,
  className,
}: {
  path: string
  className?: string
}) {
  const name = fileTypeIconName(path)
  return (
    <span
      aria-hidden="true"
      className={`inline-grid size-4 shrink-0 place-items-center ${className ?? ''}`}
    >
      <span className={FILE_TYPE_ICONS[name]} style={{ width: '100%', height: '100%' }} />
    </span>
  )
}

function fileTypeIconName(path: string): FileTypeIconName {
  const name = path.split(/[\\/]/).at(-1)?.toLocaleLowerCase() ?? path.toLocaleLowerCase()
  if (name.startsWith('readme')) return 'readme'
  if (/^(license|licence|copying)/.test(name)) return 'certificate'
  if (name.startsWith('dockerfile') || name.startsWith('compose.')) return 'docker'
  if (name === 'cmakelists.txt' || name.startsWith('cmake.')) return 'cmake'
  if (name === 'makefile' || name.startsWith('makefile.') || name === 'justfile') return 'makefile'
  if (['cargo.toml', 'cargo.lock', 'rust-toolchain.toml'].includes(name)) return 'rust'
  if (['go.mod', 'go.sum', 'go.work'].includes(name)) return 'go'
  if (name === 'pyproject.toml' || name === 'pipfile' || name.startsWith('requirements')) return 'python'
  if (['bun.lock', 'bun.lockb', 'bunfig.toml'].includes(name)) return 'bun'
  if (name.startsWith('pnpm-') || name === '.pnpmfile.cjs') return 'pnpm'
  if (name === 'yarn.lock' || name.startsWith('.yarnrc')) return 'yarn'
  if (name === 'package.json') return 'nodejs'
  if (name === 'package-lock.json') return 'npm'
  if (name === 'tsconfig.json' || name.startsWith('tsconfig.')) return 'typescript'
  if (name === 'jsconfig.json' || name.startsWith('jsconfig.')) return 'javascript'
  if (['.gitignore', '.gitattributes', '.gitmodules', '.gitconfig'].includes(name)) return 'git'
  if (name === '.editorconfig') return 'editorconfig'
  if (name.startsWith('.env')) return 'settings'
  if (name.startsWith('.prettier') || name.startsWith('prettier.config.')) return 'prettier'
  if (name.startsWith('.eslint') || name.startsWith('eslint.config.')) return 'eslint'
  if (name.startsWith('biome.json')) return 'biome'
  if (name.startsWith('.babel') || name.startsWith('babel.config.')) return 'babel'
  if (name.startsWith('.stylelint') || name.startsWith('stylelint.config.')) return 'stylelint'
  if (name.startsWith('vite.config.')) return 'vite'
  if (name.startsWith('vitest.config.') || name.startsWith('vitest.workspace.')) return 'vitest'
  if (name.startsWith('webpack.')) return 'webpack'
  if (name.startsWith('rollup.config.')) return 'rollup'
  if (name.startsWith('next.config.') || name === 'next-env.d.ts') return 'next'
  if (name.startsWith('nuxt.config.') || name === '.nuxtrc') return 'nuxt'
  if (name.startsWith('astro.config.')) return 'astro'
  if (name === 'angular.json' || name.endsWith('.component.ts')) return 'angular'
  if (name === 'nest-cli.json') return 'nest'
  if (name.startsWith('tailwind.config.')) return 'tailwindcss'
  if (name.startsWith('svelte.config.')) return 'svelte'
  if (name.startsWith('vue.config.')) return 'vue'
  if (name === 'firebase.json' || name === '.firebaserc') return 'firebase'
  if (name === 'supabase.toml') return 'supabase'
  if (name.startsWith('prisma.config.')) return 'prisma'
  if (name === 'turbo.json') return 'turborepo'
  if (name.startsWith('deno.json') || name === 'deno.lock') return 'deno'
  if (name === '.gitlab-ci.yml' || name === '.gitlab-ci.yaml') return 'gitlab'
  if (name === 'kustomization.yaml' || name === 'kustomization.yml') return 'kubernetes'
  if (name === 'chart.yaml' || name === 'values.yaml') return 'helm'
  if (name === 'nginx.conf') return 'nginx'
  if (name === '.nvmrc' || name === '.node-version') return 'nodejs'
  if (['build.gradle', 'settings.gradle', 'gradlew', 'gradlew.bat'].includes(name)) return 'gradle'
  if (name.includes('.stories.') || name.includes('.story.')) return 'storybook'
  if (name === 'gemfile' || name === 'gemfile.lock') return 'ruby'
  if (name === 'pom.xml') return 'java'

  const extension = name.includes('.') ? name.split('.').at(-1) ?? '' : ''
  if (extension === 'rs') return 'rust'
  if (['js', 'mjs', 'cjs'].includes(extension)) return 'javascript'
  if (['ts', 'mts', 'cts'].includes(extension)) return 'typescript'
  if (['jsx', 'tsx'].includes(extension)) return 'react'
  if (['py', 'pyi', 'pyw'].includes(extension)) return 'python'
  if (extension === 'go') return 'go'
  if (['c', 'h', 'm'].includes(extension)) return 'c'
  if (['cc', 'cpp', 'cxx', 'hh', 'hpp', 'hxx', 'mm'].includes(extension)) return 'cpp'
  if (extension === 'cs') return 'csharp'
  if (extension === 'swift') return 'swift'
  if (['kt', 'kts'].includes(extension)) return 'kotlin'
  if (['java', 'class'].includes(extension)) return 'java'
  if (extension === 'rb') return 'ruby'
  if (extension === 'php') return 'php'
  if (['html', 'htm'].includes(extension)) return 'html'
  if (['css', 'less'].includes(extension)) return 'css'
  if (['scss', 'sass'].includes(extension)) return 'sass'
  if (['json', 'jsonc', 'jsonl'].includes(extension)) return 'json'
  if (['yaml', 'yml'].includes(extension)) return 'yaml'
  if (['toml', 'ini', 'cfg', 'conf', 'config'].includes(extension)) return 'settings'
  if (['xml', 'xsl', 'plist'].includes(extension)) return 'xml'
  if (['md', 'mdx', 'markdown'].includes(extension)) return 'markdown'
  if (['sh', 'bash', 'zsh', 'fish'].includes(extension)) return 'console'
  if (['ps1', 'psm1'].includes(extension)) return 'powershell'
  if (['sql', 'db', 'sqlite', 'sqlite3', 'csv', 'xls', 'xlsx'].includes(extension)) return 'database'
  if (['png', 'jpg', 'jpeg', 'gif', 'webp', 'avif', 'ico', 'tiff'].includes(extension)) return 'image'
  if (extension === 'svg') return 'svg'
  if (extension === 'pdf') return 'pdf'
  if (['mp3', 'wav', 'flac', 'ogg', 'm4a'].includes(extension)) return 'audio'
  if (['mp4', 'mov', 'avi', 'webm', 'mkv'].includes(extension)) return 'video'
  if (['zip', 'gz', 'tgz', 'bz2', 'xz', '7z', 'rar', 'tar', 'jar'].includes(extension)) return 'zip'
  if (['wasm', 'wat'].includes(extension)) return 'webassembly'
  if (['svelte', 'vue', 'lua', 'dart', 'astro', 'prisma', 'xaml', 'zig', 'nix', 'proto'].includes(extension)) return extension as FileTypeIconName
  if (['tf', 'tfvars'].includes(extension)) return 'terraform'
  if (['graphql', 'gql'].includes(extension)) return 'graphql'
  if (['coffee', 'cson'].includes(extension)) return 'coffee'
  if (extension === 'cr') return 'crystal'
  if (['ex', 'exs'].includes(extension)) return 'elixir'
  if (extension === 'elm') return 'elm'
  if (['erl', 'hrl'].includes(extension)) return 'erlang'
  if (['clj', 'cljs', 'cljc', 'edn'].includes(extension)) return 'clojure'
  if (['hs', 'lhs'].includes(extension)) return 'haskell'
  if (['hx', 'hxml'].includes(extension)) return 'haxe'
  if (['jinja', 'jinja2', 'j2'].includes(extension)) return 'jinja'
  if (extension === 'jl') return 'julia'
  if (['ml', 'mli'].includes(extension)) return 'ocaml'
  if (['pl', 'pm'].includes(extension)) return 'perl'
  if (['pug', 'jade'].includes(extension)) return 'pug'
  if (['scala', 'sbt', 'sc'].includes(extension)) return 'scala'
  if (extension === 'sol') return 'solidity'
  if (['tex', 'sty', 'cls'].includes(extension)) return 'tex'
  if (['diff', 'patch'].includes(extension)) return 'diff'
  if (['exe', 'dll', 'so', 'dylib'].includes(extension)) return 'exe'
  if (extension === 'lock') return 'lock'
  return 'file'
}

const PROVIDER_ICONS: Record<ProviderKind, string> = {
  amp: 'i-flow-provider-amp',
  claude: 'i-flow-provider-claude',
  codex: 'i-flow-provider-openai',
  cursor: 'i-flow-provider-cursor',
  deepSeek: 'i-flow-provider-deepseek',
  openCode: 'i-flow-provider-opencode',
  grok: 'i-flow-provider-grok',
  pi: 'i-flow-provider-pi',
}

export const PROVIDERS: Array<{
  id: ProviderKind
  name: string
  shortName: string
  command: string
}> = [
  { id: 'amp', name: 'Amp', shortName: 'Amp', command: 'amp' },
  { id: 'claude', name: 'Claude Code', shortName: 'Claude', command: 'claude' },
  { id: 'codex', name: 'Codex CLI', shortName: 'Codex', command: 'codex' },
  { id: 'cursor', name: 'Cursor CLI', shortName: 'Cursor', command: 'cursor-agent' },
  { id: 'deepSeek', name: 'DeepSeek Harness', shortName: 'DeepSeek', command: 'dsh' },
  { id: 'openCode', name: 'OpenCode', shortName: 'OpenCode', command: 'opencode' },
  { id: 'grok', name: 'Grok Build', shortName: 'Grok', command: 'grok' },
  { id: 'pi', name: 'Pi', shortName: 'Pi', command: 'pi' },
]

export function providerMeta(provider: ProviderKind) {
  return PROVIDERS.find((candidate) => candidate.id === provider) ?? PROVIDERS[2]!
}

export function ProviderIcon({
  provider,
  className,
  label,
}: {
  provider: ProviderKind
  className?: string
  label?: string
}) {
  return (
    <span
      aria-hidden={label ? undefined : true}
      aria-label={label}
      className={`inline-grid size-4 shrink-0 place-items-center ${providerColor(provider)} ${className ?? ''}`}
      role={label ? 'img' : undefined}
    >
      <span
        aria-hidden="true"
        className={PROVIDER_ICONS[provider]}
        style={{ width: '100%', height: '100%' }}
      />
    </span>
  )
}

function providerColor(provider: ProviderKind) {
  if (provider === 'amp') return 'text-[#f34e3f]'
  if (provider === 'claude') return 'text-[#d97757]'
  if (provider === 'deepSeek') return 'text-[#4d6bfe]'
  return 'text-[#34363b] dark:text-[#f3f3f3]'
}
