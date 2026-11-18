import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';
import {copyFileSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';

const {version} = JSON.parse(readFileSync(new URL('../product.config.json', import.meta.url), 'utf8'));
assert.match(version, /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/);
const tag = `v${version}`;
const output = 'out/desktop-release';
const releasePlatforms = {
  'darwin-aarch64': {folder: 'macos', extension: '.app.tar.gz'},
  'linux-x86_64': {folder: 'appimage', extension: '.AppImage'},
  'windows-x86_64': {folder: 'nsis', extension: '-setup.exe'},
};
const basename = target => `musicOS_${version}_${target}`;
const payload = target => basename(target) + releasePlatforms[target].extension;

function releaseRepo() {
  const repo = process.env.MUSICOS_RELEASE_REPOSITORY ?? '';
  assert.match(repo, /^[A-Za-z0-9-]+\/[A-Za-z0-9_.-]+$/, 'Set MUSICOS_RELEASE_REPOSITORY to GitHub OWNER/REPO');
  return repo;
}

function publicKey() {
  const key = (process.env.MUSICOS_UPDATER_PUBLIC_KEY ?? '').trim();
  assert.match(key, /^[A-Za-z0-9+/]+={0,2}$/, 'Set MUSICOS_UPDATER_PUBLIC_KEY to the contents of the Tauri .pub file');
  assert.match(Buffer.from(key, 'base64').toString('utf8'), /^untrusted comment: [^\n]+\n[A-Za-z0-9+/]{56}\r?\n?$/);
  return key;
}

function configure(target) {
  assert.ok(Object.hasOwn(releasePlatforms, target), 'Unsupported release target');
  const config = {
    bundle: {createUpdaterArtifacts: true},
    plugins: {updater: {
      endpoints: [`https://github.com/${releaseRepo()}/releases/latest/download/latest.json`],
      pubkey: publicKey(),
      requireSignedVersion: true,
    }},
  };
  if (target === 'windows-x86_64') {
    config.bundle.resources = Object.fromEntries(
      ['onnxruntime.dll', 'onnxruntime_providers_shared.dll'].map(name => [`resources/runtime/${name}`, `runtime/${name}`]),
    );
  }
  mkdirSync(output, {recursive: true});
  writeFileSync(join(output, 'tauri.conf.json'), JSON.stringify(config, null, 2));
}

function copyBundle(root, folder, suffix, destination, signed = false) {
  const dir = join(root, folder);
  const files = readdirSync(dir).filter(name => name.endsWith(suffix));
  assert.equal(files.length, 1, `Expected one ${suffix} bundle in ${dir}`);
  const source = join(dir, files[0]);
  copyFileSync(source, destination);
  if (signed) copyFileSync(source + '.sig', destination + '.sig');
}

function stage(target) {
  assert.ok(Object.hasOwn(releasePlatforms, target), 'Unsupported release target');
  const {folder, extension} = releasePlatforms[target];
  const root = join(process.env.CARGO_TARGET_DIR || 'target', 'release', 'bundle');
  const dest = join(output, 'assets');
  mkdirSync(dest, {recursive: true});
  copyBundle(root, folder, extension, join(dest, payload(target)), true);
  if (target === 'darwin-aarch64') {
    copyBundle(root, 'dmg', '.dmg', join(dest, basename(target) + '.dmg'));
  }
}

function manifest(dir) {
  const release = `https://github.com/${releaseRepo()}/releases/download/${tag}`;
  const dmg = basename('darwin-aarch64') + '.dmg';
  const targets = Object.keys(releasePlatforms);
  const bundles = targets.map(payload);
  assert.deepEqual(readdirSync(dir).sort(), [dmg, ...bundles, ...bundles.map(name => name + '.sig')].sort(),
    'Expected only the signed release bundles and the Mac installer');
  assert.ok(statSync(join(dir, dmg)).size > 0, 'Missing Mac installer');
  const scratch = mkdtempSync(join(tmpdir(), 'musicos-signatures-'));
  const platforms = {};
  try {
    const keyPath = join(scratch, 'updater.pub');
    const sigPath = join(scratch, 'updater.minisig');
    writeFileSync(keyPath, Buffer.from(publicKey(), 'base64'));
    for (const target of targets) {
      const name = payload(target);
      const path = join(dir, name);
      const signature = readFileSync(path + '.sig', 'utf8').trim();
      const decoded = Buffer.from(signature, 'base64');
      const trusted = decoded.toString('utf8').split(/\r?\n/).find(line => line.startsWith('trusted comment: '));
      assert.ok(trusted?.split('\t').includes(`version:${version}`), `Wrong or missing signed version for ${name}`);
      writeFileSync(sigPath, decoded);
      execFileSync('minisign', ['-Vm', path, '-p', keyPath, '-x', sigPath, '-q'], {stdio: 'inherit'});
      platforms[target] = {signature, url: `${release}/${name}`};
    }
    writeFileSync(join(dir, 'latest.json'), JSON.stringify({version, platforms}, null, 2) + '\n');
  } finally {
    rmSync(scratch, {recursive: true, force: true});
  }
}

const [action, arg] = process.argv.slice(2);
if (action === 'config') configure(arg);
else if (action === 'stage') stage(arg);
else if (action === 'manifest') manifest(arg);
else if (action === 'version') console.log(version);
else throw new Error('Use config <platform>, stage <platform>, manifest <assets directory>, or version.');
