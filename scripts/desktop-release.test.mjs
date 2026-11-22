import assert from 'node:assert/strict';
import {execFileSync, spawnSync} from 'node:child_process';
import {mkdtempSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {after, test} from 'node:test';

const script = fileURLToPath(new URL('./desktop-release.mjs', import.meta.url));
const {version} = JSON.parse(readFileSync(new URL('../product.config.json', import.meta.url)));
const root = mkdtempSync(join(tmpdir(), 'musicos-release-test-'));
after(() => rmSync(root, {recursive: true, force: true}));
const secret = join(root, 'test.key');
const pub = join(root, 'test.pub');
execFileSync('minisign', ['-G', '-W', '-s', secret, '-p', pub], {stdio: 'pipe'});
const env = {...process.env, MUSICOS_RELEASE_REPOSITORY: 'HenryTheAddict/MusicOS',
  MUSICOS_UPDATER_PUBLIC_KEY: readFileSync(pub).toString('base64')};
const extensions = {'darwin-aarch64': '.app.tar.gz', 'darwin-x86_64': '.app.tar.gz',
  'linux-x86_64': '.AppImage', 'windows-x86_64': '-setup.exe'};
const name = target => `musicOS_${version}_${target}`;

function run(cwd, ...args) {
  return spawnSync(process.execPath, [script, ...args], {cwd, env, encoding: 'utf8'});
}

function fixture() {
  const dir = mkdtempSync(join(root, 'assets-'));
  for (const [target, extension] of Object.entries(extensions)) {
    const path = join(dir, name(target) + extension);
    writeFileSync(path, `fixture for ${target}`);
    const sig = path + '.sig';
    execFileSync('minisign', ['-S', '-s', secret, '-m', path, '-x', sig,
      '-t', `timestamp:0\tversion:${version}`], {stdio: 'pipe'});
    writeFileSync(sig, readFileSync(sig).toString('base64'));
    if (target.startsWith('darwin-')) writeFileSync(join(dir, name(target) + '.dmg'), 'installer');
  }
  const pkg = mkdtempSync(join(root, 'package-'));
  mkdirSync(join(pkg, 'usr/bin'), {recursive: true});
  mkdirSync(join(pkg, 'opt/musicos'), {recursive: true});
  writeFileSync(join(pkg, '.PKGINFO'), `pkgname = musicos\npkgver = ${version}-1\narch = x86_64\n`);
  writeFileSync(join(pkg, 'usr/bin/musicos'), 'launcher');
  writeFileSync(join(pkg, 'opt/musicos/AppRun'), 'application');
  execFileSync('tar', ['--zstd', '-cf', join(dir, `musicos-${version}-1-x86_64.pkg.tar.zst`),
    '-C', pkg, '.PKGINFO', 'usr/bin/musicos', 'opt/musicos/AppRun']);
  return dir;
}

test('configures all four updater targets, Mac minimum OS, and Windows runtime', () => {
  const cwd = mkdtempSync(join(root, 'config-'));
  for (const target of Object.keys(extensions)) {
    const result = run(cwd, 'config', target);
    assert.equal(result.status, 0, result.stderr);
    const config = JSON.parse(readFileSync(join(cwd, 'out/desktop-release/tauri.conf.json')));
    assert.equal(config.bundle.createUpdaterArtifacts, true);
    assert.equal(config.plugins.updater.requireSignedVersion, true);
    assert.equal(config.plugins.updater.pubkey, env.MUSICOS_UPDATER_PUBLIC_KEY);
    if (target.startsWith('darwin-')) assert.equal(config.bundle.macOS.minimumSystemVersion, '14.0');
    if (target.startsWith('windows-')) assert.equal(Object.keys(config.bundle.resources).length, 2);
  }
  assert.notEqual(run(cwd, 'config', 'unknown').status, 0);
});

test('stages signed archives and DMGs for both Mac architectures', () => {
  for (const target of ['darwin-aarch64', 'darwin-x86_64']) {
    const cwd = mkdtempSync(join(root, 'stage-'));
    for (const [folder, file] of [['macos', 'musicOS.app.tar.gz'], ['dmg', 'musicOS.dmg']]) {
      const dest = join(cwd, 'target/release/bundle', folder);
      mkdirSync(dest, {recursive: true});
      writeFileSync(join(dest, file), 'bundle');
      if (folder === 'macos') writeFileSync(join(dest, file + '.sig'), 'signature');
    }
    const result = run(cwd, 'stage', target);
    assert.equal(result.status, 0, result.stderr);
    assert.deepEqual(readdirSync(join(cwd, 'out/desktop-release/assets')).sort(),
      ['.app.tar.gz', '.app.tar.gz.sig', '.dmg'].map(ext => name(target) + ext).sort());
  }
});

test('verifies signatures and Arch metadata, emits Intel updater entry and checksums', () => {
  const dir = fixture();
  const result = run(root, 'manifest', dir);
  assert.equal(result.status, 0, result.stderr);
  const manifest = JSON.parse(readFileSync(join(dir, 'latest.json')));
  assert.deepEqual(Object.keys(manifest.platforms).sort(), Object.keys(extensions).sort());
  assert.match(manifest.platforms['darwin-x86_64'].url, /darwin-x86_64\.app\.tar\.gz$/);
  assert.equal(manifest.version, version);
  execFileSync('sha256sum', ['--check', 'SHA256SUMS'], {cwd: dir, stdio: 'pipe'});
});

test('rejects empty, missing, unexpected and tampered assets', () => {
  for (const change of ['empty', 'missing', 'unexpected', 'tampered']) {
    const dir = fixture();
    const path = join(dir, name('darwin-x86_64') + '.app.tar.gz');
    if (change === 'empty') writeFileSync(path, '');
    if (change === 'missing') rmSync(path);
    if (change === 'unexpected') writeFileSync(join(dir, 'extra'), 'extra');
    if (change === 'tampered') writeFileSync(path, 'modified payload');
    const result = run(root, 'manifest', dir);
    assert.notEqual(result.status, 0, change);
    assert.equal(readdirSync(dir).includes('latest.json'), false);
  }
});

test('rejects wrong signed versions and incorrect Arch metadata', () => {
  const dir = fixture();
  const path = join(dir, name('darwin-x86_64') + '.app.tar.gz');
  execFileSync('minisign', ['-S', '-s', secret, '-m', path, '-x', path + '.sig',
    '-t', 'timestamp:0\tversion:999.0.0'], {stdio: 'pipe'});
  writeFileSync(path + '.sig', readFileSync(path + '.sig').toString('base64'));
  const signed = run(root, 'manifest', dir);
  assert.notEqual(signed.status, 0);
  assert.match(signed.stderr, /Wrong or missing signed version/);
  const wrongArch = fixture();
  const pkg = mkdtempSync(join(root, 'wrong-arch-'));
  writeFileSync(join(pkg, '.PKGINFO'), `pkgname = musicos\npkgver = ${version}-1\narch = aarch64\n`);
  execFileSync('tar', ['--zstd', '-cf', join(wrongArch, `musicos-${version}-1-x86_64.pkg.tar.zst`),
    '-C', pkg, '.PKGINFO']);
  const result = run(root, 'manifest', wrongArch);
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Wrong Arch package metadata: arch = x86_64/);
});
