#!/usr/bin/env node

const { platform, arch } = process;
const path = require('path');
const { spawn } = require('child_process');

const platformKey = `${platform}-${arch}`;
const binName = platform === 'win32' ? 'openco.exe' : 'openco';
const binPath = path.join(__dirname, 'binaries', platformKey, binName);

const fs = require('fs');
if (!fs.existsSync(binPath)) {
    console.error(`Error: No binary found for platform "${platformKey}".`);
    console.error(`Expected: ${binPath}`);
    console.error('');
    console.error('Supported platforms: linux-x64, darwin-arm64, darwin-x64, win32-x64');
    process.exit(1);
}

const child = spawn(binPath, process.argv.slice(2), {
    stdio: 'inherit',
    windowsHide: false,
});

child.on('exit', (code) => {
    process.exit(code ?? 0);
});

child.on('error', (err) => {
    console.error('Failed to start OpenCo:', err.message);
    process.exit(1);
});
