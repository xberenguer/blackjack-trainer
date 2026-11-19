// Install Playwright normally, use NODE_PATH, or explicitly name an existing module.
const { chromium } = require(process.env.STARDUST_PLAYWRIGHT_MODULE || 'playwright');
function launchOptions(extra = {}) {
  const options = { headless: true, ...extra };
  if (process.env.STARDUST_CHROME_EXECUTABLE) options.executablePath = process.env.STARDUST_CHROME_EXECUTABLE;
  else if (process.env.STARDUST_CHROME_CHANNEL) options.channel = process.env.STARDUST_CHROME_CHANNEL;
  return options;
}
module.exports = { chromium, launchOptions };
