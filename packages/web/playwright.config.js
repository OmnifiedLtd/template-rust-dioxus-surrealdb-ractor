const path = require('path');

/** @type {import('@playwright/test').PlaywrightTestConfig} */
const config = {
  testDir: path.join(__dirname, 'e2e'),
  timeout: 120000,
  expect: {
    timeout: 10000,
  },
  use: {
    baseURL: 'http://127.0.0.1:8080',
    trace: 'on-first-retry',
  },
  webServer: {
    command: 'dx serve --package web --port 8080',
    cwd: path.resolve(__dirname, '..', '..'),
    url: 'http://127.0.0.1:8080/admin',
    reuseExistingServer: !process.env.CI,
    timeout: 120000,
  },
};

module.exports = config;
