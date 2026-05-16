import { execFileSync } from 'node:child_process'
import { cpSync, existsSync, mkdirSync, readFileSync, rmSync, symlinkSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = dirname(fileURLToPath(import.meta.url))
const root = resolve(__dirname, '..')
const tauriConfig = JSON.parse(readFileSync(join(root, 'src-tauri', 'tauri.conf.json'), 'utf8'))

const productName = tauriConfig.productName ?? 'PillArk'
const version = tauriConfig.version ?? '0.1.0'
const arch = process.arch === 'arm64' ? 'aarch64' : process.arch === 'x64' ? 'x64' : process.arch
const bundleDir = join(root, 'src-tauri', 'target', 'release', 'bundle')
const appPath = join(bundleDir, 'macos', `${productName}.app`)
const dmgDir = join(bundleDir, 'dmg')
const stagingDir = join(dmgDir, `${productName}-dmg-staging`)
const dmgPath = join(dmgDir, `${productName}_${version}_${arch}.dmg`)

if (process.platform !== 'darwin') {
  throw new Error('DMG packaging is only available on macOS.')
}

if (!existsSync(appPath)) {
  throw new Error(`App bundle not found at ${appPath}. Run "npm run build:app" first.`)
}

rmSync(stagingDir, { recursive: true, force: true })
mkdirSync(stagingDir, { recursive: true })
cpSync(appPath, join(stagingDir, `${productName}.app`), { recursive: true })
symlinkSync('/Applications', join(stagingDir, 'Applications'))
mkdirSync(dmgDir, { recursive: true })

execFileSync(
  'hdiutil',
  ['create', '-volname', productName, '-srcfolder', stagingDir, '-ov', '-format', 'UDZO', dmgPath],
  { stdio: 'inherit' },
)

rmSync(stagingDir, { recursive: true, force: true })
console.log(`DMG created at ${dmgPath}`)
