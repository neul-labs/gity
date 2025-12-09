#!/usr/bin/env node

const { execSync, spawn } = require('child_process');
const fs = require('fs');
const path = require('path');
const https = require('https');
const os = require('os');

const VERSION = '0.1.0';
const REPO = 'neul-labs/gity';

function getPlatform() {
  const platform = os.platform();
  const arch = os.arch();

  const platformMap = {
    'darwin-x64': 'x86_64-apple-darwin',
    'darwin-arm64': 'aarch64-apple-darwin',
    'linux-x64': 'x86_64-unknown-linux-gnu',
    'linux-arm64': 'aarch64-unknown-linux-gnu',
    'win32-x64': 'x86_64-pc-windows-msvc',
  };

  const key = `${platform}-${arch}`;
  const target = platformMap[key];

  if (!target) {
    console.error(`Unsupported platform: ${platform}-${arch}`);
    process.exit(1);
  }

  return { platform, target };
}

function download(url, dest) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest);
    https.get(url, (response) => {
      if (response.statusCode === 302 || response.statusCode === 301) {
        download(response.headers.location, dest).then(resolve).catch(reject);
        return;
      }
      response.pipe(file);
      file.on('finish', () => {
        file.close(resolve);
      });
    }).on('error', (err) => {
      fs.unlink(dest, () => {});
      reject(err);
    });
  });
}

async function install() {
  const { platform, target } = getPlatform();
  const ext = platform === 'win32' ? 'zip' : 'tar.gz';
  const binName = platform === 'win32' ? 'gity.exe' : 'gity';

  const url = `https://github.com/${REPO}/releases/download/v${VERSION}/gity-${VERSION}-${target}.${ext}`;
  const binDir = path.join(__dirname, '..', 'bin');
  const tmpDir = os.tmpdir();
  const archivePath = path.join(tmpDir, `gity-${VERSION}.${ext}`);

  console.log(`Downloading gity ${VERSION} for ${target}...`);

  try {
    await download(url, archivePath);

    if (!fs.existsSync(binDir)) {
      fs.mkdirSync(binDir, { recursive: true });
    }

    if (ext === 'tar.gz') {
      execSync(`tar -xzf "${archivePath}" -C "${binDir}"`, { stdio: 'inherit' });
    } else {
      execSync(`unzip -o "${archivePath}" -d "${binDir}"`, { stdio: 'inherit' });
    }

    const binPath = path.join(binDir, binName);
    if (platform !== 'win32') {
      fs.chmodSync(binPath, 0o755);
    }

    fs.unlinkSync(archivePath);
    console.log('gity installed successfully!');
  } catch (error) {
    console.error('Failed to install gity:', error.message);
    console.error('You can install manually from: https://github.com/neul-labs/gity/releases');
    process.exit(1);
  }
}

install();
