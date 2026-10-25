import { test, expect, type Page } from "@playwright/test";
test("SuperCommand searches, creates playlists and supports Escape",async({page})=>{
  await start(page);
  await page.keyboard.press('ControlOrMeta+k');
  const dialog=page.getByRole('dialog',{name:'SuperCommand'});
  await expect(dialog).toBeVisible();
  await dialog.getByRole('combobox').fill('Discovery');
  await expect(dialog.getByRole('option').filter({hasText:'Album · Daft Punk'})).toBeVisible();
  await dialog.getByRole('option').filter({hasText:'Album · Daft Punk'}).click();
  await expect(page.getByRole('heading',{name:'Discovery',exact:true})).toBeVisible();
  await page.keyboard.press('ControlOrMeta+k');
  await dialog.getByRole('combobox').fill('create playlist');
  await page.keyboard.press('Enter');
  await dialog.getByRole('combobox',{name:'Playlist name'}).fill('Command mix');
  await page.keyboard.press('Enter');
  await expect(dialog).not.toBeVisible();
  await expect(page.locator('.personal-library')).toContainText('Command mix');
  await page.keyboard.press('ControlOrMeta+k');
  await page.keyboard.press('Escape');
  await expect(dialog).not.toBeVisible();
});
test('player audio tooltip escapes header clipping and stays inside compact windows',async({page})=>{
  await start(page);
  await page.getByRole('button',{name:'Open Discovery',exact:true}).click();
  await page.getByRole('button',{name:'Play collection',exact:true}).click();
  for(const size of [{width:1280,height:800},{width:720,height:480}]){
    await page.setViewportSize(size);
    await page.locator('.now-playing .audio-quality button').hover();
    const popup=page.locator('[id^=audio-specs-player-]');
    await expect(popup).toBeVisible();
    await expect(popup).toContainText('44.1 kHz');
    expect(await popup.evaluate(e=>e.matches(':popover-open'))).toBe(true);
    const box=await popup.boundingBox();
    expect(box!.x).toBeGreaterThanOrEqual(0);expect(box!.y).toBeGreaterThanOrEqual(0);
    expect(box!.x+box!.width).toBeLessThanOrEqual(size.width);
    expect(box!.y+box!.height).toBeLessThanOrEqual(size.height);
    await page.keyboard.press('Escape');
    await expect(popup).not.toBeVisible();
    await page.locator('.album-copy h1').hover();
  }
});
test("right-click actions and quality specifications follow the album",async({page})=>{
  await start(page);
  await page.getByRole('button',{name:'Open Discovery',exact:true}).click();
  const quality=page.locator('.album-copy .audio-quality');
  await quality.hover();
  await expect(quality.getByRole('tooltip')).toContainText('44.1 kHz');
  await expect(quality.getByRole('tooltip')).toBeVisible();
  const header=await quality.boundingBox();
  const hero=await page.locator('.album-hero').boundingBox();
  expect(header!.y).toBeGreaterThanOrEqual(hero!.y);
  await page.locator('.track-row').first().click({button:'right'});
  await expect(page.getByRole('menuitem',{name:'Play',exact:true})).toBeVisible();
  await page.keyboard.press('Escape');
});
test("library folders can be removed and restored",async({page})=>{
  await start(page);await settings(page);
  await page.getByRole('tab',{name:'Library',exact:true}).click();
  await page.getByRole('button',{name:'Remove from library',exact:true}).click();
  await expect(page.getByRole('button',{name:'Restore folder',exact:true})).toBeVisible();
  await page.getByRole('button',{name:'Restore folder',exact:true}).click();
  await expect(page.getByRole('button',{name:'Remove from library',exact:true})).toBeVisible();
});
async function start(page: Page, extra = "") {
  await page.goto(`/?fixture=1${extra}`);
  await expect(
    page.getByRole("button", { name: "Open Plastic Beach", exact: true }),
  ).toBeVisible();
}
async function settings(page: Page) {
  await page.getByRole("button", { name: "Choose section" }).click();
  await page.getByRole("menuitem", { name: "Settings", exact: true }).click();
}
async function library(page: Page) {
  await page.getByRole("button", { name: "Choose section" }).click();
  await page.getByRole("menuitem", { name: "Library", exact: true }).click();
}
test("album browsing does not play; artist link, album strip and back restore location", async ({
  page,
}) => {
  await page.setViewportSize({ width: 720, height: 480 });
  await start(page);
  await page.locator(".page-scroll").evaluate((e) => (e.scrollTop = 120));
  await page
    .getByRole("button", { name: "Open Discovery", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "Discovery", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText("Nothing playing yet", { exact: true }),
  ).toBeVisible();
  await page
    .locator(".album-copy")
    .getByRole("button", { name: "Daft Punk", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "Daft Punk", exact: true }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Random Access Memories", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "Random Access Memories", exact: true }),
  ).toBeVisible();
  await page.keyboard.press("Alt+ArrowLeft");
  await expect(
    page.getByRole("heading", { name: "Daft Punk", exact: true }),
  ).toBeVisible();
  await page.keyboard.press("Alt+ArrowLeft");
  await expect(
    page.getByRole("heading", { name: "Discovery", exact: true }),
  ).toBeVisible();
  await page.keyboard.press("Alt+ArrowLeft");
  await expect(
    page.getByRole("button", { name: "Open Discovery", exact: true }),
  ).toBeVisible();
  expect(
    await page.locator(".page-scroll").evaluate((e) => e.scrollTop),
  ).toBeGreaterThan(80);
});
test("artists and genres open the selected collection", async ({ page }) => {
  await start(page);
  await page.getByRole("button", { name: "Artists", exact: true }).click();
  await page.getByRole("button", { name: "Open M83", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "M83", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Fantasy", exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Genres", exact: true }).click();
  await page
    .getByRole("button", { name: "Open Electronic", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "Electronic", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Open Plastic Beach", exact: true }),
  ).toBeVisible();
});
test("playback follows selected sequence and queue", async ({ page }) => {
  await start(page);
  await page
    .getByRole("button", { name: "Open Plastic Beach", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Play Second Light", exact: true })
    .first()
    .click();
  await expect(page.locator(".now-playing-info strong")).toHaveText(
    "Second Light",
  );
  await page.getByRole("button", { name: "Next track", exact: true }).click();
  await expect(page.locator(".now-playing-info strong")).toHaveText(
    "A Long Way Home Through the City at Night",
  );
  await page
    .getByRole("button", { name: "Previous track", exact: true })
    .click();
  await expect(page.locator(".now-playing-info strong")).toHaveText(
    "Second Light",
  );
  await page.getByRole("button", { name: "Open Up next", exact: true }).click();
  await expect(page.locator(".queue-row")).toHaveCount(11);
});
for (const percent of [80, 100, 125, 150])
  test(`size ${percent}% has usable scaled controls and no horizontal overflow at both sizes`, async ({
    page,
  }) => {
    await start(page);
    await settings(page);
    await page
      .getByRole("button", { name: `${percent}%`, exact: true })
      .click();
    await expect(
      page.getByRole("button", { name: `${percent}%`, exact: true }),
    ).toHaveAttribute("aria-pressed", "true");
    for (const size of [
      { width: 720, height: 480 },
      { width: 1280, height: 800 },
    ]) {
      await page.setViewportSize(size);
      await expect(page.getByRole("switch", { name: "Coverflow Colors", exact: true })).toBeVisible();
      const metrics = await page.evaluate(() => ({
        font: getComputedStyle(document.documentElement).fontSize,
        width: document.querySelector(".app-shell")!.scrollWidth,
        viewport: innerWidth,
      }));
      expect(parseFloat(metrics.font)).toBeCloseTo((16 * percent) / 100);
      expect(metrics.width).toBeLessThanOrEqual(metrics.viewport);
      await expect.poll(() => page.locator(".page-content").evaluate(e => e.getAnimations({subtree:true}).length)).toBe(0);
    await page.screenshot({
        path: `out/rebuild-qa/settings-${size.width}-${percent}.png`,
      });
    }
    await library(page);
    await page.getByRole("button", { name: "Artists", exact: true }).click();
    await page.getByRole("button", { name: "Open M83", exact: true }).click();
    await expect(
      page.getByRole("heading", { name: "M83", exact: true }),
    ).toBeVisible();
  });
test("settings update name immediately and persist after reload", async ({
  page,
}) => {
  await start(page);
  await settings(page);
  await page.getByRole("tab", {name:"General",exact:true}).click();
  const field = page.getByRole("textbox", { name: "Name", exact: true });
  await field.fill("  Ada Lovelace  ");
  await page.getByRole("tab",{name:"Appearance",exact:true}).click();
  await page.getByRole("button", { name: "125%", exact: true }).click();
  await page.getByRole("switch", { name: "Coverflow Colors", exact: true }).check();
  await library(page);
  await expect(
    page.getByRole("heading", { name: "Ada Lovelace's stuff", exact: true }),
  ).toBeVisible();
  await page.reload();
  await settings(page);
  await page.getByRole("tab",{name:"General",exact:true}).click();
  await expect(field).toHaveValue("Ada Lovelace");
  await page.getByRole("tab",{name:"Appearance",exact:true}).click();
  await expect(page.getByRole("switch", { name: "Coverflow Colors", exact: true })).toBeChecked();
  await expect(
    page.getByRole("button", { name: "125%", exact: true }),
  ).toHaveAttribute("aria-pressed", "true");
  await page.getByRole("tab",{name:"General",exact:true}).click();
  await field.fill("   ");
  await expect(page.getByRole("alert")).toContainText("Enter a name.");
});
test("default blue, adaptive theme, and disabling colors restore blue", async ({
  page,
}) => {
  await start(page);
  await expect(page.locator(".current-theme")).toHaveCSS(
    "background-color",
    "rgb(0, 174, 255)",
  );
  await settings(page);
  await page.getByRole("switch", { name: "Coverflow Colors", exact: true }).check();
  await library(page);
  await page
    .getByRole("button", { name: "Open Plastic Beach", exact: true })
    .click();
  await expect(page.locator(".current-theme")).toHaveCSS(
    "background-color",
    "rgb(243, 247, 249)",
  );
  await settings(page);
  await page.getByRole("switch", { name: "Coverflow Colors", exact: true }).uncheck();
  await library(page);
  await expect(page.locator(".current-theme")).toHaveCSS(
    "background-color",
    "rgb(0, 174, 255)",
  );
});
test("Discover is removed and library categories stay separate from Back",async({page})=>{
 await start(page);await expect(page.getByRole("button",{name:"Discover",exact:true})).toHaveCount(0);
 await page.getByRole("button",{name:"Artists",exact:true}).click();await page.getByRole("button",{name:"Open M83",exact:true}).click();
 await expect(page.getByRole("button",{name:"Artists",exact:true})).toBeVisible();
 await page.getByRole("button",{name:"Back to Artists",exact:true}).click();await expect(page.getByRole("button",{name:"Open M83",exact:true})).toBeVisible();
});
test("10,000 songs remain virtualized while scrolling", async ({ page }) => {
  await start(page, "&large=1");
  await page.getByRole("button", { name: "Songs", exact: true }).click();
  await expect(
    page.getByRole("table", { name: "Songs", exact: true }),
  ).toBeVisible();
  expect(await page.locator(".track-row").count()).toBeLessThan(70);
  await page
    .locator(".page-scroll")
    .evaluate((e) => (e.scrollTop = e.scrollHeight - 600));
  await expect(
    page.getByText("Library song 9999", { exact: true }),
  ).toBeVisible();
  expect(await page.locator(".track-row").count()).toBeLessThan(70);
});
test("album and artist screenshots at Sketch and desktop sizes", async ({
  page,
}) => {
  await start(page);
  for (const size of [
    { width: 720, height: 480 },
    { width: 1280, height: 800 },
  ]) {
    await page.setViewportSize(size);
    await page.getByRole("button", { name: "Albums", exact: true }).click();
    await page
      .getByRole("button", { name: "Open Plastic Beach", exact: true })
      .click();
    await expect.poll(() => page.locator(".page-content").evaluate(e => e.getAnimations({subtree:true}).length)).toBe(0);
    await page.screenshot({ path: `out/rebuild-qa/album-${size.width}.png` });
    await page.getByRole("button", { name: "Artists", exact: true }).click();
    await page.getByRole("button", { name: "Open M83", exact: true }).click();
    await expect(page.getByText("Wikipedia ↗", { exact: true })).toBeVisible();
    await expect.poll(() => page.locator(".page-content").evaluate(e => e.getAnimations({subtree:true}).length)).toBe(0);
    await page.screenshot({ path: `out/rebuild-qa/artist-${size.width}.png` });
  }
});

test("large songs route restores scroll after an artist visit", async ({
  page,
}) => {
  await start(page, "&large=1");
  await page.getByRole("button", { name: "Songs", exact: true }).click();
  await page.locator(".page-scroll").evaluate((e) => (e.scrollTop = 5000));
  await expect
    .poll(() => page.locator(".track-row").count())
    .toBeGreaterThan(0);
  const visible = await page.locator(".track-row").evaluateAll((rows) => {
    const viewport = document
      .querySelector(".page-scroll")!
      .getBoundingClientRect();
    return rows.findIndex((row) => {
      const r = row.getBoundingClientRect();
      return r.top >= viewport.top && r.bottom <= viewport.bottom;
    });
  });
  await page
    .locator(".track-row")
    .nth(visible)
    .locator(".track-title .text-link")
    .click();
  await page.keyboard.press("Alt+ArrowLeft");
  await expect
    .poll(() => page.locator(".page-scroll").evaluate((e) => e.scrollTop))
    .toBeGreaterThan(4500);
});

test("a large playback queue stays virtualized", async ({ page }) => {
  await start(page, "&large=1");
  await page.getByRole("button", { name: "Songs", exact: true }).click();
  await page.locator(".track-row").first().hover();
  await page.locator(".row-play").first().click();
  await page.getByRole("button", { name: "Open Up next", exact: true }).click();
  expect(await page.locator(".queue-row").count()).toBeLessThan(100);
  await page
    .locator(".queue-scroll")
    .evaluate((e) => (e.scrollTop = e.scrollHeight - 400));
  await expect(
    page.locator(".queue-row").getByText("Library song 9999", { exact: true }),
  ).toBeVisible();
});

test("detail pages fit both window sizes at every interface size", async ({
  page,
}) => {
  await start(page);
  for (const percent of [80, 100, 125, 150]) {
    await settings(page);
    await page
      .getByRole("button", { name: `${percent}%`, exact: true })
      .click();
    await library(page);
    for (const width of [720, 1280]) {
      await page.setViewportSize({ width, height: width === 720 ? 480 : 800 });
      await page.getByRole("button", { name: "Albums", exact: true }).click();
      await page
        .getByRole("button", { name: "Open Plastic Beach", exact: true })
        .click();
      await expect(
        page.getByRole("button", { name: "Play collection", exact: true }),
      ).toBeVisible();
      expect(
        await page
          .locator(".page-scroll")
          .evaluate((e) => e.scrollWidth - e.clientWidth),
      ).toBeLessThanOrEqual(1);
      const overlap = await page.locator(".album-copy").evaluate((copy) => {
        const artist = copy
          .querySelector(".artist-link")!
          .getBoundingClientRect();
        const controls = copy
          .querySelector(".collection-actions")!
          .getBoundingClientRect();
        return (
          artist.left < controls.right &&
          artist.right > controls.left &&
          artist.top < controls.bottom &&
          artist.bottom > controls.top
        );
      });
      expect(overlap).toBe(false);
      await expect.poll(() => page.locator(".page-content").evaluate(e => e.getAnimations({subtree:true}).length)).toBe(0);
    await page.screenshot({
        path: `out/rebuild-qa/album-${width}-${percent}.png`,
      });
      await page
        .locator(".album-copy")
        .getByRole("button", { name: "Gorillaz", exact: true })
        .click();
      await expect(
        page.getByRole("heading", { name: "Gorillaz", exact: true }),
      ).toBeVisible();
      expect(
        await page
          .locator(".page-scroll")
          .evaluate((e) => e.scrollWidth - e.clientWidth),
      ).toBeLessThanOrEqual(1);
      await expect.poll(() => page.locator(".page-content").evaluate(e => e.getAnimations({subtree:true}).length)).toBe(0);
    await page.screenshot({
        path: `out/rebuild-qa/artist-${width}-${percent}.png`,
      });
    }
  }
});

test("Space activates a focused album without starting playback", async ({
  page,
}) => {
  await start(page);
  await page
    .getByRole("button", { name: "Open Plastic Beach", exact: true })
    .focus();
  await page.keyboard.press("Space");
  await expect(
    page.getByRole("heading", { name: "Plastic Beach", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText("Nothing playing yet", { exact: true }),
  ).toBeVisible();
  await expect(page.getByRole("alert")).toHaveCount(0);
});

test("compact desktop preserves Sketch header and artist composition", async ({ page }) => {
  await page.setViewportSize({width:720,height:480});
  await start(page);
  const selector = await page.getByRole("button", {name:"Choose section", exact:true}).boundingBox();
  const player = await page.locator(".now-playing").boundingBox();
  expect(selector!.x).toBeCloseTo(130, 0);
  expect(selector!.width).toBeCloseTo(222, 0);
  expect(player!.x).toBeCloseTo(368, 0);
  await page.getByRole("button", {name:"Artists", exact:true}).click();
  await page.getByRole("button", {name:"Open M83", exact:true}).click();
  await expect(page.locator(".artist-portrait-button > .cover")).toHaveCSS("transform", "none");
  const portrait = await page.locator(".artist-portrait-button > .cover").boundingBox();
  expect(portrait!.width).toBeCloseTo(192, 0);
  const title = await page.getByRole("heading", {name:"M83", exact:true}).boundingBox();
  // Short names must stay readable above the portrait rather than disappear behind it.
  expect(portrait!.y).toBeGreaterThanOrEqual(title!.y + title!.height);
  const strip = await page.locator(".album-strip").boundingBox();
  expect(strip!.y).toBeGreaterThan(portrait!.y + portrait!.height);
  expect(strip!.y).toBeLessThan(480);
});

test("wide windows retain Sketch album sizes and sort menu works", async ({page}) => {
  await start(page);
  await page.getByRole("button", {name:"Sort albums", exact:true}).click();
  await page.getByRole("menuitem", {name:"Album Name", exact:true}).click();
  await expect(page.locator(".card-open strong").first()).toHaveText("AM");
  await page.getByRole("button", {name:"Open Plastic Beach", exact:true}).click();
  await expect(page.locator(".album-hero > .cover")).toHaveCSS("transform", "none");
  await expect(page.locator(".track-table")).toHaveCSS("transform", "none");
  const geometry = await page.evaluate(() => ({
    cover: document.querySelector(".album-hero > .cover")!.getBoundingClientRect().width,
    title: getComputedStyle(document.querySelector(".album-copy h1")!).fontSize,
    row: document.querySelector(".album-detail .track-row")!.getBoundingClientRect().top,
    player: document.querySelector(".now-playing")!.getBoundingClientRect().left,
    selectorRight: document.querySelector(".section-control > .menu-trigger:first-child")!.getBoundingClientRect().right,
    brandRight: document.querySelector(".header-brand")!.getBoundingClientRect().right,
    selectorLeft: document.querySelector(".section-control > .menu-trigger:first-child")!.getBoundingClientRect().left,
  }));
  expect(geometry.cover).toBe(128);
  expect(geometry.title).toBe("32px");
  expect(geometry.row).toBeCloseTo(240,0);
  expect(geometry.player).toBeGreaterThan(geometry.selectorRight);
  expect(geometry.brandRight).toBeLessThanOrEqual(geometry.selectorLeft);
});

test("track reveal keeps titles and rows steady and remains keyboard accessible", async ({ page }) => {
  await page.setViewportSize({ width: 720, height: 480 });
  await start(page);
  await page.getByRole("button", { name: "Open Plastic Beach", exact: true }).click();
  const row = page.locator(".track-row").nth(1);
  const label = row.locator(".track-label");
  const play = row.getByRole("button", { name: "Play Second Light", exact: true });
  await page.mouse.move(120, 40);
  await expect(play).toHaveCSS("opacity", "0");
  await expect(page.locator(".track-table")).toHaveCSS("transform", "none");
  const before = await row.boundingBox();
  const initial = await label.boundingBox();
  await row.hover();
  await expect(play).toHaveCSS("opacity", "1");
  await expect.poll(async () => (await label.boundingBox())!.x - initial!.x).toBeCloseTo(0, 0);
  expect(await row.boundingBox()).toEqual(before);
  await page.mouse.move(120, 40);
  await play.focus();
  await expect(play).toHaveCSS("opacity", "1");
  await page.keyboard.press("Enter");
  await expect(page.locator(".now-playing-info strong")).toHaveText("Second Light");
  await expect(play).toHaveCSS("opacity", "1");
  // A playback update must not re-run the whole page entrance or reset scroll.
  await page.locator(".page-scroll").evaluate(e => e.scrollTop = 100);
  await page.getByRole("button", { name: "Pause", exact: true }).click();
  expect(await page.locator(".page-scroll").evaluate(e => e.scrollTop)).toBe(100);
  expect(await page.locator(".page-content").evaluate(e => e.getAnimations({subtree:true}).some(a => a.id === "page-entrance"))).toBe(false);
});

test("reduced motion keeps reveal and navigation immediate with no entrance animations", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.setViewportSize({ width: 720, height: 480 });
  await start(page);
  await page.getByRole("button", { name: "Open Plastic Beach", exact: true }).click();
  const row = page.locator(".track-row").first();
  await row.hover();
  await expect(row.locator(".row-play")).toHaveCSS("opacity", "1");
  expect(await page.locator(".page-content").evaluate(e => e.getAnimations({subtree:true}).length)).toBe(0);
  await page.locator(".artist-link").filter({hasText:"Gorillaz"}).click();
  await expect(page.getByRole("heading", { name: "Gorillaz", exact: true })).toBeVisible();
  expect(await page.locator(".page-content").evaluate(e => e.getAnimations({subtree:true}).length)).toBe(0);
});

for (const virtualized of [false, true]) {
  test(`${virtualized ? "1000-album" : "small"} grid fills its width and remeasures after resize at every interface size`, async ({ page }) => {
    await page.goto(`/?fixture=1${virtualized ? "&largeAlbums=1" : ""}`);
    await expect(page.locator(".collection-card").first()).toBeVisible();
    for (const percent of [80, 100, 125, 150]) {
      await settings(page);
      await page.getByRole("tab", {name:"Appearance",exact:true}).click();
      await page.getByRole("button", { name: `${percent}%`, exact: true }).click();
      await library(page);
      for (const width of [720, 860, 1024, 1280, 720]) {
        await page.setViewportSize({ width, height: width === 720 ? 480 : 800 });
        const assertGeometry = async () => {
          const geometry = await page.locator(".collection-grid").first().evaluate(grid => {
            const cards = Array.from(grid.querySelectorAll(".collection-card"));
            const bounds = grid.getBoundingClientRect();
            const first = cards[0]?.getBoundingClientRect();
            const row = cards.filter(card => Math.abs(card.getBoundingClientRect().top - first!.top) < 1);
            const covers = row.map(card => card.querySelector(".cover")!.getBoundingClientRect());
            const caption = cards[0]?.querySelector(".card-caption")?.getBoundingClientRect();
            return { width: bounds.width, left: first!.left - bounds.left, right: bounds.right - row.at(-1)!.getBoundingClientRect().right, square: covers.every(c => Math.abs(c.width - c.height) < 1), equal: covers.every(c => Math.abs(c.width - covers[0].width) < 1), cover: covers[0].width, caption: caption!.width, card: first!.width, overflow: document.querySelector(".page-scroll")!.scrollWidth - document.querySelector(".page-scroll")!.clientWidth };
          });
          expect(geometry.left).toBeLessThan(1);
          // A complete row uses the available width, with no stranded empty column.
          expect(geometry.right).toBeLessThan(1);
          expect(geometry.square && geometry.equal).toBe(true);
          expect(geometry.cover).toBeCloseTo(geometry.card, 0);
          expect(geometry.caption).toBeCloseTo(geometry.card, 0);
          expect(geometry.cover).toBeGreaterThanOrEqual(111 * percent / 100);
          expect(geometry.cover).toBeLessThanOrEqual(170 * percent / 100);
          expect(geometry.overflow).toBeLessThanOrEqual(1);
        };
        await expect(assertGeometry).toPass();
      }
    }
    if (virtualized) {
      expect(await page.locator(".collection-card").count()).toBeLessThan(100);
      await page.locator(".page-scroll").evaluate(e => e.scrollTop = e.scrollHeight * .7);
      await page.setViewportSize({ width: 1024, height: 800 });
      await expect.poll(() => page.locator(".virtual-grid").count()).toBeGreaterThan(2);
      await expect(async () => {
      const rows = await page.locator(".virtual-grid").evaluateAll(rows => rows.map(row => {
        const card = row.querySelector(".collection-card")!.getBoundingClientRect();
        return { top: card.top, bottom: card.bottom };
      }).sort((a,b) => a.top - b.top));
      for (let index = 1; index < rows.length; index++) expect(rows[index].top - rows[index - 1].bottom).toBeGreaterThanOrEqual(10);
      }).toPass();
    }
    await expect(page.getByRole("menu")).toHaveCount(0);
    await page.screenshot({ path: `out/rebuild-qa/grid-responsive-${virtualized ? "large" : "small"}.png` });
  });
}

test("selected Sketch vectors morph on playback and settle exactly after rapid changes", async ({ page }) => {
  await start(page);
  await page.getByRole("button", { name: "Open Plastic Beach", exact: true }).click();
  await page.getByRole("button", { name: "Play collection", exact: true }).click();
  const glyph = page.locator('.transport [data-icon]');
  const pause = page.locator('.transport [data-icon="pause"]');
  await expect(pause).toBeVisible();
  await expect(pause).not.toHaveAttribute("data-morphing", "true");
  const exactPause = await pause.locator("path").last().getAttribute("d");
  expect(exactPause).toContain("M21.2,4");
  await pause.evaluate(node => {
    const samples: string[] = [];
    const observer = new MutationObserver(records => {
      for (const record of records) if (record.attributeName === "d") samples.push((record.target as Element).getAttribute("d") ?? "");
      node.setAttribute("data-samples", JSON.stringify(samples));
    });
    observer.observe(node, { attributes: true, subtree: true, attributeFilter: ["d"] });
    (node as HTMLElement & { stopObserving?: () => void }).stopObserving = () => observer.disconnect();
  });
  await page.getByRole("button", { name: "Pause", exact: true }).click();
  await expect(page.locator('.transport [data-icon="play"]')).toBeVisible();
  await page.getByRole("button", { name: "Play", exact: true }).click();
  await expect(pause).not.toHaveAttribute("data-morphing", "true");
  expect(await pause.locator("path").last().getAttribute("d")).toBe(exactPause);
  const samples = JSON.parse(await pause.getAttribute("data-samples") ?? "[]") as string[];
  expect(new Set(samples).size).toBeGreaterThan(3);
  expect(samples.every(d => !d.includes("NaN") && !d.includes("Infinity"))).toBe(true);
  await pause.evaluate(node => (node as HTMLElement & { stopObserving?: () => void }).stopObserving?.());
  // Local SVG ids cannot resolve to another icon's clipping/gradient definition.
  const ids = await page.locator(".asset-icon [id]").evaluateAll(nodes => nodes.map(n => n.id));
  expect(new Set(ids).size).toBe(ids.length);
  expect(await glyph.count()).toBe(3);
  await page.screenshot({ path: "out/rebuild-qa/asset-icons-playback.png" });
});

test("SVG playback morph respects reduced motion", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await start(page);
  await page.getByRole("button", { name: "Open Plastic Beach", exact: true }).click();
  await page.getByRole("button", { name: "Play collection", exact: true }).click();
  const pause = page.locator('.transport [data-icon="pause"]');
  await expect(pause).toBeVisible();
  expect(await pause.getAttribute("data-morphing")).toBeNull();
  expect(await pause.locator("path").last().getAttribute("d")).toContain("M21.2,4");
});

test("MusicOS branding, Figtree parentheses and opt-in Labs", async ({page}) => {
  await start(page);
  await expect(page.locator(".header-brand")).toContainText("MusicOS");
  await page.evaluate(() => document.fonts.ready);
  const metrics=await page.evaluate(() => {
    const canvas=document.createElement("canvas"); const ctx=canvas.getContext("2d")!;
    ctx.font='16px "MusicOS Sans"';
    return {loaded:document.fonts.check('16px "MusicOS Sans"'), left:ctx.measureText("(").width, right:ctx.measureText(")").width, space:ctx.measureText(" ").width};
  });
  expect(metrics.loaded).toBe(true);
  expect(Math.abs(metrics.left-metrics.right)).toBeLessThan(.1);
  expect(metrics.left).toBeLessThan(metrics.space*2);
  await settings(page);
  await page.getByRole("tab",{name:"Integrations",exact:true}).click();
  await expect(page.getByRole("heading",{name:/MusicOS Labs/})).toBeVisible();
  const discord=page.getByRole("switch",{name:"Discord presence",exact:true});
  await expect(discord).not.toBeChecked();
  await expect(page.getByRole("switch",{name:"Last.fm scrobbling",exact:true})).toBeDisabled();
  await discord.click();
  await expect(page.getByRole("alert")).toContainText("Application ID");
  await expect(discord).not.toBeChecked();
  await page.getByText("Set up Discord",{exact:true}).click();
  await page.getByLabel("Application ID",{exact:true}).fill("123456789012345678");
  await page.getByRole("button",{name:"Save",exact:true}).click();
  await discord.check();
  await expect(discord).toBeChecked();
  await discord.uncheck();
  await expect(discord).not.toBeChecked();
  const stored=await page.evaluate(()=>JSON.stringify(localStorage));
  expect(stored).not.toMatch(/shared_secret|session_key|api_key/);
});

test("grid resize animations finish and reduced motion disables reflow", async ({page}) => {
  await page.setViewportSize({width:1280,height:800});
  await page.goto("/?fixture=1&largeAlbums=1");
  await expect(page.locator(".collection-card").first()).toBeVisible();
  await page.setViewportSize({width:720,height:480});
  await expect.poll(()=>page.locator(".page-content").evaluate(e=>e.getAnimations({subtree:true}).length)).toBe(0);
  await page.emulateMedia({reducedMotion:"reduce"});
  await page.setViewportSize({width:1280,height:800});
  await expect.poll(()=>page.locator(".page-content").evaluate(e=>e.getAnimations({subtree:true}).filter(a=>a.id==="grid-reflow").length)).toBe(0);
  await expect(page.getByRole("button",{name:"Albums",exact:true})).toBeVisible();
});

test("Sketch smooth corners resize correctly and playback progress fills to seek position", async ({page}) => {
  await page.setViewportSize({width:720,height:480});
  await start(page);
  await expect(page.locator(".page-frame")).toHaveCSS("clip-path","none");
  await expect(page.locator(".card-open .cover").first()).toHaveAttribute("data-corner-smoothing","1");
  const before=await page.locator(".card-open .cover").first().evaluate(e=>getComputedStyle(e).clipPath);
  await page.setViewportSize({width:1280,height:800});
  await expect.poll(()=>page.locator(".card-open .cover").first().evaluate(e=>getComputedStyle(e).clipPath)).not.toBe(before);
  await page.getByRole("button",{name:"Open Plastic Beach",exact:true}).click();
  await page.getByRole("button",{name:"Play collection",exact:true}).click();
  const slider=page.getByRole("slider",{name:"Playback position",exact:true});
  await expect(slider).toBeEnabled();
  await slider.focus();await slider.press("End");
  await expect.poll(()=>slider.inputValue()).toBe(await slider.getAttribute("max"));
  await expect.poll(()=>page.locator(".progress-rail > span").evaluate(el=>Math.abs(el.getBoundingClientRect().width-el.parentElement!.getBoundingClientRect().width))).toBeLessThan(1);
  await slider.press("Home");await expect.poll(()=>slider.inputValue()).toBe("0");
  await expect.poll(()=>page.locator(".progress-rail > span").evaluate(el=>el.getBoundingClientRect().width)).toBe(0);
  await page.screenshot({path:"out/rebuild-qa/figtree-smooth-album-1280.png"});
});

test("artist notes unfold on hover and focus, settle, and respect Escape",async({page})=>{
 await start(page);await page.getByRole("button",{name:"Artists",exact:true}).click();await page.getByRole("button",{name:"Open M83",exact:true}).click();
 const portrait=page.getByRole("button",{name:"Expand M83 portrait",exact:true});const facts=page.locator(".portrait-fact");
 await portrait.hover();await expect(facts.first()).toHaveCSS("opacity","1");await expect(portrait).toHaveAttribute("aria-expanded","true");
 await expect.poll(()=>page.locator(".artist-hero").evaluate(e=>e.getAnimations({subtree:true}).length)).toBe(0);
 await page.screenshot({path:"out/rebuild-qa/artist-notes-1280.png"});
 await portrait.press("Escape");await expect(facts.first()).toHaveCSS("opacity","0");
 await page.mouse.move(0,0);await portrait.focus();await expect(facts.first()).toHaveCSS("opacity","1");
 await page.emulateMedia({reducedMotion:"reduce"});await expect(facts.first()).toHaveCSS("transition-duration","0s");
});
test("Settings categories support keyboard navigation, volume and tab restoration",async({page})=>{
 await start(page);await settings(page);
 const appearance=page.getByRole("tab",{name:"Appearance",exact:true});await expect(appearance).toHaveAttribute("aria-selected","true");await appearance.focus();await appearance.press("ArrowRight");
 await expect(page.getByRole("tab",{name:"Audio",exact:true})).toHaveAttribute("aria-selected","true");
 await expect(page.getByRole("slider",{name:/Volume/})).toBeEnabled();
 await page.getByRole("slider",{name:/Volume/}).focus();await page.getByRole("slider",{name:/Volume/}).press("Home");await expect(page.getByRole("slider",{name:/Volume/})).toHaveValue("0");
 await library(page);await settings(page);await expect(appearance).toHaveAttribute("aria-selected","true");
 await appearance.focus();await appearance.press("End");await expect(page.getByRole("textbox",{name:"Name",exact:true})).toBeVisible();
});

test("management controls are intrinsic and batch edits change only chosen metadata",async({page})=>{
 await start(page);
 await settings(page);await page.getByRole('tab',{name:'Library',exact:true}).click();await page.getByRole('button',{name:'Metadata & artwork',exact:true}).click();
 await expect(page.getByRole('tab',{name:'Library',exact:true})).toHaveAttribute('aria-selected','true');
 await page.getByRole('tab',{name:'Edit songs',exact:true}).click();
 await page.getByRole('textbox',{name:'Find songs to edit',exact:true}).fill('Plastic Beach');
 await page.getByRole('button',{name:'Select matching songs',exact:true}).click();
 await expect(page.getByText('12 selected',{exact:true})).toBeVisible();
 await expect(page.getByRole('button',{name:'Review changes to 12 songs',exact:true})).toBeDisabled();
 await page.getByRole('combobox',{name:'Artist change',exact:true}).selectOption('set');
 await page.getByRole('textbox',{name:'New artist',exact:true}).fill('Edited artist');
 await page.getByRole('button',{name:'Review changes to 12 songs',exact:true}).click();
 await page.getByRole('button',{name:'Apply changes to 12 songs',exact:true}).click();
 await expect(page.getByRole('status').filter({hasText:'Saved metadata for 12 songs.'})).toBeVisible();
 await library(page);await page.getByRole('button',{name:'Songs',exact:true}).click();
 await page.getByRole('textbox',{name:'Search',exact:true}).fill('Edited artist');
 await expect(page.locator('.track-row')).toHaveCount(12);
 await page.screenshot({path:'out/rebuild-qa/batch-edited-songs.png'});
});

test("search follows artwork with finite focus motion and artist notes have no visible label",async({page})=>{
 await start(page);await settings(page);await page.getByRole('switch',{name:'Coverflow Colors',exact:true}).check();await library(page);
 const search=page.locator('.search');const input=page.getByRole('textbox',{name:'Search',exact:true});
 await input.focus();await expect.poll(()=>search.evaluate(e=>e.getAnimations({subtree:true}).length)).toBe(0);
 await expect(search).not.toHaveCSS('background-color','rgb(141, 159, 255)');
 await page.getByRole('button',{name:'Artists',exact:true}).click();await page.getByRole('button',{name:'Open M83',exact:true}).click();
 await expect(page.getByText('Artist notes',{exact:true})).toHaveCount(0);
 await expect(page.locator('.artist-detail .track-row').first()).not.toHaveCSS('background-color','rgb(0, 154, 255)');
 const covers=await page.locator('.album-strip > button').evaluateAll(nodes=>nodes.map(n=>n.getBoundingClientRect()));
 for(let i=1;i<covers.length;i++)expect(covers[i].left).toBeGreaterThanOrEqual(covers[i-1].right);
 await expect.poll(()=>page.locator('.page-content').evaluate(e=>e.getAnimations({subtree:true}).length)).toBe(0);
 await page.screenshot({path:'out/rebuild-qa/library-polish-artist.png'});
 await page.emulateMedia({reducedMotion:'reduce'});await page.getByRole('button',{name:'Artists',exact:true}).click();await input.focus();await expect.poll(()=>search.evaluate(e=>e.getAnimations({subtree:true}).length)).toBe(0);
});

test("playlists persist and rule playlists follow favorite changes",async({page})=>{
 await start(page);await page.getByRole('button',{name:'Create playlist',exact:true}).click();
 const create=page.getByRole('dialog',{name:'Create playlist',exact:true});await create.getByRole('textbox',{name:'Name',exact:true}).fill('Night drive');
 await create.getByRole('button',{name:'Create playlist',exact:true}).click();
 await expect(page.getByRole('heading',{name:'Night drive',exact:true})).toBeVisible();
 await page.getByRole('button',{name:'Edit playlist',exact:true}).click();
 await page.getByRole('textbox',{name:'Playlist description'}).fill('After dark');
 await page.getByRole('button',{name:'Save details',exact:true}).click();
 await expect(page.locator('.playlist-hero')).toContainText('After dark');
 await page.getByRole('button',{name:'Add songs',exact:true}).click();
 await page.getByRole('searchbox',{name:'Find playlist songs'}).fill('Opening');
 await page.getByRole('checkbox',{name:'Select Opening',exact:true}).first().check();
 await page.getByRole('button',{name:'Add selected',exact:true}).click();
 await expect(page.locator('.playlist-hero')).toContainText('1 songs');
 await page.getByRole('button',{name:'Edit playlist',exact:true}).click();
 await page.getByRole('button',{name:'Favorite Opening',exact:true}).click();
 await expect(page.getByRole('button',{name:'Unfavorite Opening',exact:true})).toHaveAttribute('aria-pressed','true');
 await page.getByRole('button',{name:'Manage collections',exact:true}).click();await page.getByRole('tab',{name:/Smart playlists/}).click();await page.getByRole('button',{name:'Create Smart playlist',exact:true}).click();
 await page.getByRole('textbox',{name:'Name',exact:true}).fill('My hearts');await page.getByRole('button',{name:'Rule 1 field',exact:true}).click();await page.getByRole('menuitemradio',{name:'favorited',exact:true}).click();await page.getByRole('button',{name:'Save Smart playlist'}).click();
 const spark=page.locator('.hub-collection-open').filter({hasText:'My hearts'});await spark.click();
 await expect(page.getByRole('heading',{name:'My hearts',exact:true})).toBeVisible();
 await page.getByRole('button',{name:'Unfavorite Opening',exact:true}).click();
 await expect(page.getByRole('button',{name:'Unfavorite Opening',exact:true})).toHaveCount(0);
 await page.reload();await expect(page.getByRole('heading',{name:'My hearts',exact:true})).toBeVisible();
 await expect(page.locator('.personal-library .collection-open').filter({hasText:'Night drive'})).toBeVisible();
});
test('queue reordering, clearing and DJmode preserve the playing song',async({page})=>{
 await start(page);await page.getByRole('button',{name:'Open Discovery',exact:true}).click();await page.getByRole('button',{name:'Play collection',exact:true}).click();
 await page.getByRole('button',{name:'Open Up next',exact:true}).click();
 const rows=page.locator('.queue-row');const before=await rows.locator('strong').allTextContents();
 await rows.nth(1).getByRole('button',{name:`Play ${before[1]} from queue`,exact:true}).focus();await page.keyboard.press('Alt+ArrowUp');
 await expect(rows.nth(0).locator('strong')).toHaveText(before[1]);
 await expect(page.locator('.now-playing-info > strong')).toHaveText('Opening');
 await page.locator('.now-playing').getByRole('button',{name:/Audio quality/}).click();await page.getByRole('switch',{name:'DJmode',exact:true}).check();await expect(page.getByRole('switch',{name:'DJmode',exact:true})).toBeChecked();
 await expect(page.getByRole('slider',{name:'Volume',exact:true})).toHaveCSS('appearance','none');
 await page.keyboard.press('Escape');await page.getByRole('button',{name:'Clear upcoming',exact:true}).click();await expect(rows).toHaveCount(0);await expect(page.locator('.now-playing-info > strong')).toHaveText('Opening');
});
test('standalone heart settles after rapid toggles and honors reduced motion',async({page})=>{
 await start(page);await page.getByRole('button',{name:'Open Discovery',exact:true}).click();
 const heart=page.locator('.track-favorite').first();const initial=await heart.getAttribute('aria-pressed');
 await heart.click({clickCount:3,delay:35});
 await expect(heart).toHaveAttribute('aria-pressed',initial==='true'?'false':'true');
 await expect(heart).toHaveCSS('background-color','rgba(0, 0, 0, 0)');await expect(heart).toHaveCSS('border-radius','0px');
 await expect.poll(()=>heart.evaluate(e=>e.getAnimations({subtree:true}).length)).toBe(0);
 expect(await heart.locator('.heart-motion').evaluate(e=>e.getBoundingClientRect().width)).toBeGreaterThan(15);
 await page.emulateMedia({reducedMotion:'reduce'});await heart.click();
 await expect.poll(()=>heart.evaluate(e=>e.getAnimations({subtree:true}).length)).toBe(0);
});
test('playlist editing reorders without changing playback and survives restart',async({page})=>{
 await start(page);await page.getByRole('button',{name:'Create playlist',exact:true}).click();const create=page.getByRole('dialog',{name:'Create playlist',exact:true});await create.getByRole('textbox',{name:'Name',exact:true}).fill('Ordered mix');await create.getByRole('button',{name:'Create playlist',exact:true}).click();await page.getByRole('button',{name:'Edit playlist',exact:true}).click();await page.getByRole('button',{name:'Add songs',exact:true}).click();
 const checks=page.locator('.playlist-edit-row input[type=checkbox]');await checks.nth(0).check();await checks.nth(1).check();await checks.nth(2).check();await page.getByRole('button',{name:'Add selected',exact:true}).click();await page.getByRole('button',{name:'Playlist songs',exact:true}).click();
 const names=await page.locator('.playlist-edit-row strong').allTextContents();await page.locator('.playlist-edit-row').nth(1).getByRole('button',{name:/^Move .* up$/}).click();await expect(page.locator('.playlist-edit-row').first().locator('strong')).toHaveText(names[1]);await page.getByRole('button',{name:'Edit playlist',exact:true}).click();await page.getByRole('button',{name:'Play collection',exact:true}).click();
 await page.getByRole('button',{name:'Edit playlist',exact:true}).click();await page.getByRole('button',{name:'Add songs',exact:true}).click();await checks.nth(4).check();await page.getByRole('button',{name:'Add selected',exact:true}).click();await page.getByRole('button',{name:'Open Up next',exact:true}).click();await expect(page.locator('.queue-row')).toHaveCount(2);await page.getByRole('button',{name:'Close Up next',exact:true}).click();
 await page.reload();await expect(page.getByRole('heading',{name:'Ordered mix',exact:true})).toBeVisible();await expect(page.locator('.playlist-hero')).toContainText('4 songs');
});
test("biography expands and seek rail thickens without a thumb",async({page})=>{
 await start(page);await page.getByRole('button',{name:'Artists',exact:true}).click();await page.getByRole('button',{name:'Open M83',exact:true}).click();await page.getByRole('button',{name:'Read more',exact:true}).click();
 await expect(page.locator('.artist-biography')).toHaveCSS('max-height','none');await expect(page.locator('.artist-biography')).toContainText('Their records include');await page.getByRole('button',{name:'Read less',exact:true}).click();
 await page.getByRole('button',{name:'Play collection',exact:true}).click();const rail=page.locator('.progress-rail');const before=await rail.evaluate(el=>el.getBoundingClientRect().height);await page.locator('.playback-progress').hover();await expect.poll(()=>rail.evaluate(el=>el.getBoundingClientRect().height)).toBeGreaterThan(before);
 await expect(page.locator('.seek')).toHaveCSS('opacity','0');
});
test('playlist and queue layouts fit both sizes at every interface scale',async({page})=>{
 await start(page);await page.getByRole('button',{name:'Create playlist',exact:true}).click();const creation=page.getByRole('dialog',{name:'Create playlist',exact:true});await creation.getByRole('textbox',{name:'Name',exact:true}).fill('A very long playlist name for the tracks that last all night');await creation.getByRole('button',{name:'Create playlist',exact:true}).click();
 for(const size of [{width:720,height:480},{width:1280,height:800}])for(const scale of [80,100,125,150]){
  await page.setViewportSize(size);await settings(page);await page.getByRole('tab',{name:'Appearance',exact:true}).click();await page.getByRole('button',{name:`${scale}%`,exact:true}).click();await library(page);
  await expect(page.getByRole('heading',{name:'A very long playlist name for the tracks that last all night',exact:true})).toBeVisible();
  expect(await page.locator('.page-scroll').evaluate(e=>e.scrollWidth-e.clientWidth)).toBeLessThanOrEqual(1);
  await page.getByRole('button',{name:'Edit playlist',exact:true}).click();await page.getByRole('button',{name:'Add songs',exact:true}).click();
  await expect(page.getByRole('searchbox',{name:'Find playlist songs'})).toBeVisible();
  expect(await page.locator('.playlist-editor').evaluate(e=>e.scrollWidth-e.clientWidth)).toBeLessThanOrEqual(1);
  await page.getByRole('button',{name:'Edit playlist',exact:true}).click();await page.getByRole('button',{name:'Open Up next',exact:true}).click();
  const queue=page.locator('.queue-panel');const box=await queue.boundingBox();expect(box!.x).toBeGreaterThanOrEqual(0);expect(box!.x+box!.width).toBeLessThanOrEqual(size.width+1);expect(box!.y+box!.height).toBeLessThanOrEqual(size.height+1);
  await page.getByRole('button',{name:'DJmode Off',exact:true}).click();await expect(page.getByRole('button',{name:'DJmode On',exact:true})).toBeVisible();await page.getByRole('button',{name:'DJmode On',exact:true}).click();
  await page.screenshot({path:`out/rebuild-qa/playlist-queue-${size.width}-${scale}.png`});await page.getByRole('button',{name:'Close Up next',exact:true}).click();
 }
});

test('artist selection has no card panel and volume uses custom icons and controls',async({page})=>{
 await start(page);
 await page.getByRole('button',{name:'Artists',exact:true}).click();
 const card=page.locator('.collection-card[data-artist-name]').first();
 await card.locator('.card-open').focus();
 await expect(card).toHaveCSS('background-color','rgba(0, 0, 0, 0)');
 await expect(card.locator('small')).toContainText(/\d+ albums? · \d+ songs?/);
 await page.screenshot({path:'out/rebuild-qa/artist-grid-no-panel-1280.png'});
 await page.getByRole('button',{name:'Albums',exact:true}).click();
 await page.getByRole('button',{name:'Open Discovery',exact:true}).click();
 await page.getByRole('button',{name:'Play collection',exact:true}).click();
 await page.getByRole('button',{name:'Open Up next',exact:true}).click();
 await expect(page.locator('[data-icon="DJmode"] svg')).toBeVisible();
 await expect(page.locator('.audio-panel [data-icon="AudioBot"] svg')).toHaveCount(0);
 const volume=page.getByRole('slider',{name:'Volume',exact:true});
 await volume.focus();await page.keyboard.press('Home');await expect(volume).toHaveValue('0');
 await page.getByRole('button',{name:'Unmute',exact:true}).click();await expect(volume).not.toHaveValue('0');
 await page.setViewportSize({width:720,height:480});
 await page.screenshot({path:'out/rebuild-qa/queue-volume-icons-720.png'});
});
