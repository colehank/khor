/** Functional flows use the pill's accessible overview action. Actual touch
 * trajectories and timing are exercised by check-content-toolbar and the Lab. */
export async function openTabs(page,band=page.locator('[data-deck-band]:visible').last()) {
  const title=band.locator('button[data-deck-title]:visible').first();
  await (await title.count()?title:band).press('ArrowUp');
  return true;
}
