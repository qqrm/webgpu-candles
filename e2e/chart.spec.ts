import { expect, Page, test } from '@playwright/test';

type MarketRequests = {
  urls: string[];
  webSocketUrls: string[];
  historyCount: number;
};

const intervalDuration = (interval: string): number => {
  const durations: Record<string, number> = {
    '1s': 1_000,
    '1m': 60_000,
    '5m': 300_000,
    '15m': 900_000,
    '1h': 3_600_000,
    '1d': 86_400_000,
    '1w': 604_800_000,
    '1M': 2_592_000_000,
  };
  return durations[interval] ?? 60_000;
};

const kline = (timestamp: number, index: number, empty: boolean, basePrice: number) => {
  const center = basePrice + index * 0.04 + Math.sin(index / 9) * basePrice * 0.0002;
  const close = empty ? center : center + Math.cos(index / 5) * 3;
  const high = empty ? center : Math.max(center, close) + 2;
  const low = empty ? center : Math.min(center, close) - 2;
  const volume = empty ? 0 : 1 + (index % 7) / 10;
  return [
    timestamp,
    center.toFixed(2),
    high.toFixed(2),
    low.toFixed(2),
    close.toFixed(2),
    volume.toFixed(4),
    timestamp,
    '0',
    1,
    '0',
    '0',
    '0',
  ];
};

async function mockMarket(page: Page): Promise<MarketRequests> {
  const requests: MarketRequests = { urls: [], webSocketUrls: [], historyCount: 0 };

  await page.route(/https:\/\/api\.binance\.com\/api\/v3\/(uiKlines|klines).*/, async route => {
    const url = new URL(route.request().url());
    const interval = url.searchParams.get('interval') ?? '1m';
    const symbol = url.searchParams.get('symbol') ?? 'BTCUSDT';
    const basePrice = symbol === 'ETHUSDT' ? 3_200 : symbol === 'SOLUSDT' ? 150 : 50_000;
    const limit = Number(url.searchParams.get('limit') ?? 1_000);
    const duration = intervalDuration(interval);
    const endTime = Number(url.searchParams.get('endTime') ?? Date.now());
    const alignedEnd = Math.floor(endTime / duration) * duration;
    const first = alignedEnd - (limit - 1) * duration;
    const isHistory = url.searchParams.has('endTime');
    requests.urls.push(url.toString());
    if (isHistory) requests.historyCount += 1;

    const rows = Array.from({ length: limit }, (_, index) => {
      const timestamp = first + index * duration;
      const emptyTwoSecondBucket =
        interval === '1s' && Math.floor(timestamp / 2_000) % 10 === 0;
      return kline(timestamp, index, emptyTwoSecondBucket, basePrice);
    });

    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      headers: { 'access-control-allow-origin': '*' },
      body: JSON.stringify(rows),
    });
  });

  await page.routeWebSocket(/wss:\/\/stream\.binance\.com.*/, socket => {
    const url = new URL(socket.url());
    requests.webSocketUrls.push(url.toString());
    const symbol = url.pathname.split('/').at(-1)?.split('@')[0] ?? 'btcusdt';
    const price = symbol === 'ethusdt' ? 3_204 : symbol === 'solusdt' ? 154 : 50_404;
    const interval = url.pathname.split('kline_')[1] ?? '1m';
    const duration = intervalDuration(interval);
    const timestamp = Math.floor(Date.now() / duration) * duration;
    setTimeout(() => {
      socket.send(
        JSON.stringify({
          k: {
            t: timestamp,
            o: price.toFixed(2),
            h: (price + 2).toFixed(2),
            l: (price - 2).toFixed(2),
            c: (price + 1).toFixed(2),
            v: '2.5000',
          },
        }),
      );
    }, 50);
  });

  return requests;
}

function captureRuntimeErrors(page: Page): string[] {
  const errors: string[] = [];
  page.on('console', message => {
    if (message.type() === 'error') errors.push(message.text());
  });
  page.on('pageerror', error => errors.push(error.message));
  return errors;
}

async function openReadyChart(page: Page) {
  await page.goto('/');
  await expect(page.locator('.connection-pill')).toHaveText(/LIVE/);
  await expect(page.locator('.metric-value').first()).not.toHaveText('0');
  await expect(page.locator('#chart-canvas')).toBeVisible();
}

async function dragRight(page: Page, repetitions: number) {
  const canvas = page.locator('#chart-canvas');
  const box = await canvas.boundingBox();
  if (!box) throw new Error('Chart canvas has no bounding box');
  for (let index = 0; index < repetitions; index += 1) {
    await page.mouse.move(box.x + box.width * 0.2, box.y + box.height * 0.5);
    await page.mouse.down();
    await page.mouse.move(box.x + box.width * 0.9, box.y + box.height * 0.5, { steps: 5 });
    await page.mouse.up();
  }
}

test('loads WebGPU chart and keeps controls internally consistent', async ({ page }) => {
  const market = await mockMarket(page);
  const errors = captureRuntimeErrors(page);
  await openReadyChart(page);
  await expect.poll(() => market.urls.length).toBe(3);

  await expect(page.locator('.market-price')).toHaveText(/^\$\d+$/);
  await expect(page.getByText('Real-time updates')).toHaveCount(0);
  await expect(page.getByText('WebSocket LIVE')).toHaveCount(0);

  const scaleBefore = await page.locator('.price-level').allTextContents();
  await page.getByRole('checkbox', { name: 'SMA20', exact: true }).uncheck();
  await expect(page.getByRole('checkbox', { name: 'SMA20', exact: true })).not.toBeChecked();
  expect(await page.locator('.price-level').allTextContents()).toEqual(scaleBefore);

  const requestCountBeforeZoom = market.urls.length;
  for (let index = 0; index < 16; index += 1) {
    await page.getByRole('button', { name: 'Zoom in' }).click();
  }
  await expect(page.locator('.metric-value').last()).toHaveText('4.0×');
  expect(market.urls).toHaveLength(requestCountBeforeZoom);

  const timeBefore = await page.locator('.time-scale').innerText();
  await dragRight(page, 1);
  await expect.poll(() => page.locator('.time-scale').innerText()).not.toBe(timeBefore);
  expect(errors).toEqual([]);
});

test('keeps all three market streams hot and switches the rendered chart instantly', async ({
  page,
}) => {
  const market = await mockMarket(page);
  const errors = captureRuntimeErrors(page);
  await openReadyChart(page);

  await expect.poll(() => new Set(market.webSocketUrls).size).toBe(3);
  for (const symbol of ['btcusdt', 'ethusdt', 'solusdt']) {
    expect(market.webSocketUrls.some(url => url.includes(`${symbol}@kline_1m`))).toBe(true);
    expect(market.urls.some(url => url.includes(`symbol=${symbol.toUpperCase()}`))).toBe(true);
  }

  const socketsBeforeSwitch = market.webSocketUrls.length;
  await page.getByRole('button', { name: 'ETHUSDT', exact: true }).click();
  await expect(page.locator('.market-symbol')).toHaveText('ETHUSDT · SPOT');
  await expect(page.locator('.market-price')).toHaveText(/^\$3\d{3}$/);
  await expect(page.locator('.price-level').first()).toHaveText(/^3\d{3}$/);
  await expect(page.locator('.connection-pill')).toHaveText(/LIVE/);

  await page.getByRole('button', { name: 'SOLUSDT', exact: true }).click();
  await expect(page.locator('.market-symbol')).toHaveText('SOLUSDT · SPOT');
  await expect(page.locator('.market-price')).toHaveText(/^\$1\d{2}$/);
  await expect(page.locator('.price-level').first()).toHaveText(/^1\d{2}$/);

  await page.getByRole('button', { name: 'BTCUSDT', exact: true }).click();
  await expect(page.locator('.market-symbol')).toHaveText('BTCUSDT · SPOT');
  await expect(page.locator('.market-price')).toHaveText(/^\$5\d{4}$/);
  expect(market.webSocketUrls).toHaveLength(socketsBeforeSwitch);
  expect(errors).toEqual([]);
});

test('builds real 2s candles from Binance 1s data and removes empty buckets', async ({ page }) => {
  const market = await mockMarket(page);
  const errors = captureRuntimeErrors(page);
  await openReadyChart(page);

  await page.getByRole('button', { name: '2s', exact: true }).click();
  await expect.poll(() => market.urls.some(url => url.includes('interval=1s'))).toBe(true);
  await expect(page.locator('.connection-pill')).toHaveText(/LIVE/);
  await expect
    .poll(async () => Number(await page.locator('.metric-value').first().innerText()))
    .toBeGreaterThanOrEqual(450);
  await expect
    .poll(async () => Number(await page.locator('.metric-value').first().innerText()))
    .toBeLessThanOrEqual(451);
  await expect(page.getByText(/Failed to load/i)).toHaveCount(0);
  expect(errors).toEqual([]);
});

test('loads older history only after reaching the left edge', async ({ page }) => {
  const market = await mockMarket(page);
  const errors = captureRuntimeErrors(page);
  await openReadyChart(page);

  const initialCount = Number(await page.locator('.metric-value').first().innerText());
  expect(initialCount).toBe(1_000);
  expect(market.historyCount).toBe(0);

  await dragRight(page, 14);
  await expect.poll(() => market.historyCount).toBeGreaterThan(0);
  await expect
    .poll(async () => Number(await page.locator('.metric-value').first().innerText()))
    .toBeGreaterThan(initialCount);
  await expect(page.getByText(/Failed to load/i)).toHaveCount(0);
  expect(errors).toEqual([]);
});
