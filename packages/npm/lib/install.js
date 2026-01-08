#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const https = require('https');
const { createHash } = require('crypto');

const platform = process.platform;
const arch = process.arch;
const version = require('../package.json').version;

const BINARY_MAP = {
  'linux-x64': {
    file: `gity-${version}-x86_64-unknown-linux-gnu.tar.gz`,
    sha256: ''  // Will be filled by CI
  },
  'linux-arm64': {
    file: `gity-${version}-aarch64-unknown-linux-gnu.tar.gz`,
    sha256: ''
  },
  'darwin-x64': {
    file: `gity-${version}-x86_64-apple-darwin.tar.gz`,
    sha256: ''
  },
  'darwin-arm64': {
    file: `gity-${version}-aarch64-apple-darwin.tar.gz`,
    sha256: ''
  },
  'win32-x64': {
    file: `gity-${version}-x86_64-pc-windows-msvc.zip`,
    sha256: ''
  }
};

const key = `${platform}-${arch}`;
const info = BINARY_MAP[key];

if (!info) {
  console.log(`No binary available for ${platform} ${arch}, skipping binary download`);
  process.exit(0);
}

const baseUrl = `https://github.com/neul-labs/gity/releases/download/v${version}`;
const binaryDir = path.join(__dirname, '..', 'binaries', key);
const archivePath = path.join(binaryDir, info.file);
const tempPath = archivePath + '.tmp';

function downloadFile(url, dest) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest);
    https.get(url, (response) => {
      if (response.statusCode === 302 || response.statusCode === 301) {
        downloadFile(response.headers.location, dest).then(resolve).catch(reject);
        return;
      }
      response.pipe(file);
      file.on('finish', () => {
        file.close();
        resolve();
      });
    }).on('error', (err) => {
      fs.unlink(dest, () => {});
      reject(err);
    });
  });
}

async function install() {
  console.log(`Downloading Gity ${version} for ${platform} ${arch}...`);

  fs.mkdirSync(binaryDir, { recursive: true });

  const url = `${baseUrl}/${info.file}`;
  await downloadFile(url, tempPath);

  // Verify checksum if provided
  if (info.sha256) {
    const hash = createHash('sha256');
    const fileStream = fs.createReadStream(tempPath);
    await new Promise((resolve, reject) => {
      fileStream.on('data', data => hash.update(data));
      fileStream.on('end', () => {
        const checksum = hash.digest('hex');
        if (checksum !== info.sha256) {
          fs.unlinkSync(tempPath);
          reject(new Error(`Checksum mismatch: expected ${info.sha256}, got ${checksum}`));
        } else {
          resolve();
        }
      });
      fileStream.on('error', reject);
    });
    console.log('Checksum verified');
  }

  fs.renameSync(tempPath, archivePath);

  // Extract archive
  if (platform === 'win32') {
    const AdmZip = require('adm-zip');
    const zip = new AdmZip(archivePath);
    zip.extractAllTo(binaryDir, true);
  } else {
    const tar = require('tar');
    await tar.extract({
      file: archivePath,
      cwd: binaryDir
    });
  }

  // Clean up archive
  fs.unlinkSync(archivePath);

  console.log(`Installed Gity binary to ${binaryDir}`);
}

install().catch((err) => {
  console.error(`Installation failed: ${err.message}`);
  process.exit(1);
});
