#!/usr/bin/env node

const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

const platform = process.platform;
const arch = process.arch;
const version = require('../package.json').version;

const BINARY_MAP = {
  'linux-x64': 'gity',
  'linux-arm64': 'gity',
  'darwin-x64': 'gity',
  'darwin-arm64': 'gity',
  'win32-x64': 'gity.exe'
};

function getBinaryPath() {
  const key = `${platform}-${arch}`;
  const binaryName = BINARY_MAP[key];
  if (!binaryName) {
    console.error(`Error: No binary available for ${platform} ${arch}`);
    process.exit(1);
  }

  const binDir = path.join(__dirname, '..', 'binaries', `${platform}-${arch}`);
  return path.join(binDir, binaryName);
}

function ensureBinary() {
  const binaryPath = getBinaryPath();
  if (fs.existsSync(binaryPath)) {
    return binaryPath;
  }
  console.error(`Error: Binary not found at ${binaryPath}`);
  console.error('Please run "npm install" first to download the binary.');
  process.exit(1);
}

function main() {
  const binaryPath = ensureBinary();

  // Make binary executable on Unix
  if (platform !== 'win32') {
    fs.chmodSync(binaryPath, 0o755);
  }

  const args = process.argv.slice(2);
  const child = spawn(binaryPath, args, {
    stdio: 'inherit',
    cwd: process.cwd()
  });

  child.on('error', (err) => {
    console.error(`Failed to execute: ${err.message}`);
    process.exit(1);
  });

  child.on('exit', (code) => {
    process.exit(code);
  });
}

main();
