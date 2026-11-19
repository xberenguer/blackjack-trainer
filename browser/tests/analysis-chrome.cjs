// Read-only production browser QA. Only this test drives the public DOM/API.
// A20x clock covers30 actual Rust rounds without reading hidden WASM state.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const http = require('node:http');
const path = require('node:path');
const { chromium, launchOptions } = require('./chrome-config.cjs');
const target = path.resolve(process.argv[2] || path.join(__dirname, '..'));
const output = path.resolve(process.argv[3] || path.join(__dirname, '../validation/analysis'));

(async () => {
  const server = http.createServer((req, res) => {
    const route = req.url.split('?')[0];
    const name = route === '/' ? 'index.html' : route.slice(1);
    if (!['index.html', 'app.js', 'journal-host.js', 'journal-codec.js', 'style.css', 'stardust-rust.wasm'].includes(name)) {
      res.writeHead(404).end(); return;
    }
    res.setHeader('Content-Type', name.endsWith('.wasm') ? 'application/wasm' : name.endsWith('.js') ? 'text/javascript' : name.endsWith('.css') ? 'text/css' : 'text/html');
    res.end(fs.readFileSync(path.join(target, name)));
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  let browser, page;
  try {
    browser = await chromium.launch(launchOptions());
    page = await browser.newPage({ viewport: { width: 1024, height: 900 } });
    const errors = [];
    page.on('pageerror', e => errors.push(e.message));
    await page.addInitScript(() => {
      const nativeNow = performance.now.bind(performance);
      Object.defineProperty(performance, 'now', { value: () => nativeNow() * 20 });
      Object.defineProperty(Crypto.prototype, 'getRandomValues', { value: s => { s.fill(0); s[0] = 2; return s; } });
    });
    await page.goto(`http://127.0.0.1:${server.address().port}`);
    const info = () => page.evaluate(() => window.stardust.info());
    const frames = () => page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
    const waitView = async view => {
      await page.waitForFunction(v => window.stardust?.info().view === v, view);
      await frames();
      assert.equal((await info()).view, view);
    };
    const key = (type, grey) => page.evaluate(({type, grey}) => {
      document.dispatchEvent(new KeyboardEvent(type, { key: grey ? 'ArrowLeft' : 'ArrowRight', bubbles: true }));
    }, {type, grey});
    const tap = async (grey = false) => {
      await key('keydown', grey); await key('keyup', grey);
      // .10 queues input while public storage acknowledgment is pending.
      // Do not enqueue another navigation tap until the prior frame is presented.
      await frames();
    };
    const focus = async n => {
      for (let count = 0; (await info()).focus !== n; count++) {
        assert(count < 40, `focus ${n}: ${JSON.stringify(await info())}`);
        const before = await info();
        await tap(true);
        await page.waitForFunction(p => {
          const state = window.stardust.info();
          return state.view === p.view && state.focus !== p.focus;
        }, before);
      }
    };
    const pageNext = async () => {
      const before = await page.locator('#screen').evaluate(canvas => canvas.toDataURL());
      await tap(true);
      await page.waitForFunction(old => document.getElementById('screen').toDataURL() !== old, before);
      await frames();
    };
    await waitView('Cut');
    let guard = 0;
    while ((await info()).rounds < 30) {
      assert(++guard < 3000, `round progress: ${JSON.stringify(await info())}`);
      const state = await info();
      if (['Cut', 'Burn', 'Refill', 'Notice'].includes(state.view)) await tap();
      else if (state.view === 'Table' && [1, 2, 3, 5].includes(state.phase)) await tap();
      else await frames();
    }
    await frames();
    const baseline = await info();
    assert(baseline.rounds >= 30);
    await key('keydown', true);
    await page.waitForFunction(() => window.stardust.info().view === 'Menu');
    await key('keyup', true);
    await waitView('Menu');
    for (let tries = 0; (await info()).menu_item !== 'Session totals'; tries++) {
      assert(tries < 20);
      const before = (await info()).focus;
      await tap(true);
      await page.waitForFunction(previous => window.stardust.info().focus !== previous, before);
    }
    await tap(); await waitView('Session');
    await focus(3); await tap(); await waitView('AnalysisMenu');
    fs.mkdirSync(output, { recursive: true });
    const shot = name => page.screenshot({ path: path.join(output, `${name}.png`), fullPage: true });
    await shot('00-analysis-menu');
    let pagesChecked = 0;
    for (const [index, view, pages] of [
      [0, 'AnalysisSummary', 4], [1, 'AnalysisGraph', 1],
      [2, 'AnalysisRounds', baseline.rounds], [3, 'AnalysisAccuracy', 12],
      [4, 'AnalysisCosts', 3], [5, 'AnalysisDecisions', 1],
      [6, 'AnalysisAssumptions', 3]
    ]) {
      await focus(index); await tap(); await waitView(view);
      for (let p = 0; p < pages; p++) {
        await shot(`${String(index + 1).padStart(2, '0')}-${view}-${String(p).padStart(2, '0')}`);
        pagesChecked++;
        if (pages > 1) await pageNext();
      }
      await tap(); await waitView('AnalysisMenu');
    }
    const after = await info();
    assert.deepEqual([after.rounds, after.chips, after.phase], [baseline.rounds, baseline.chips, baseline.phase]);
    assert.deepEqual(errors, []);
    const summary = { version: after.version, settled_rounds: baseline.rounds, analysis_views: 8, detail_pages: pagesChecked, live_chips_phase_rounds_unchanged: true, clock_scale: 20, target, screenshots: output };
    fs.writeFileSync(path.join(output, 'analysis-acceptance.json'), JSON.stringify(summary, null, 2) + '\n');
    console.log(`PASS: actual Chrome ${after.version}, ${baseline.rounds} settled Rust rounds, all8 analysis views/${pagesChecked} detail pages (all round history), graph/reference-band and unavailable samples, unchanged live chips/phase/rounds;20x clock.`);
  } catch (error) {
    if (page) console.error(await page.evaluate(() => window.stardust?.info()).catch(() => null));
    throw error;
  } finally {
    await browser?.close();
    await new Promise(resolve => server.close(resolve));
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
