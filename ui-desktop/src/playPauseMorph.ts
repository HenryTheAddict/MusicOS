const ns = 'http://www.w3.org/2000/svg';
type Point = [number, number];
// Both contours begin as the complete Play silhouette. They separate into the
// Pause bars, so no bar has to grow out of an unrelated point or fade in abruptly.
const play: Point[] = [[4, 5], [4, 27], [26.764, 16], [26.764, 16]];
const pause: Point[][] = [
  [[6, 4], [6, 28], [14, 28], [14, 4]],
  [[18, 4], [18, 28], [26, 28], [26, 4]],
];
function rounded(points: Point[], radius: number) {
  const corners = points.map((p, i) => {
    // Skip a coincident triangle vertex when measuring its rounded corner.
    let before = points[(i + 3) % 4], after = points[(i + 1) % 4];
    if (Math.hypot(before[0] - p[0], before[1] - p[1]) < .001) before = points[(i + 2) % 4];
    if (Math.hypot(after[0] - p[0], after[1] - p[1]) < .001) after = points[(i + 2) % 4];
    const a = Math.hypot(before[0] - p[0], before[1] - p[1]);
    const b = Math.hypot(after[0] - p[0], after[1] - p[1]);
    const r = Math.min(radius, a / 2, b / 2);
    return {p, entry: p.map((v, j) => v + (before[j] - v) * r / (a || 1)), exit: p.map((v, j) => v + (after[j] - v) * r / (b || 1))};
  });
  // The duplicated tip contributes one curve, never a doubled-back segment.
  const distinct = corners.filter((c, i) => !i || Math.hypot(c.p[0] - corners[i-1].p[0], c.p[1] - corners[i-1].p[1]) > .001);
  return `M${distinct[0].entry.join(',')} ` + distinct.map((c, i) => `Q${c.p.join(',')} ${c.exit.join(',')} L${distinct[(i + 1) % distinct.length].entry.join(',')}`).join(' ') + ' Z';
}

export function playPauseMorph(host: HTMLElement, toPause: boolean, finalMarkup: string) {
  const value = Number(host.dataset.transportMorphProgress ?? (toPause ? 0 : 1));
  const start = Number.isFinite(value) ? Math.min(1,Math.max(0,value)) : (toPause ? 0 : 1);
  const end = toPause ? 1 : 0;
  const svg = document.createElementNS(ns, 'svg');
  svg.setAttribute('viewBox', '0 0 32 32');
  svg.setAttribute('aria-hidden', 'true');
  svg.dataset.transportMorph = 'true';
  const template=document.createElement('template');template.innerHTML=finalMarkup;
  const defs=template.content.querySelector('defs');if(defs)svg.append(defs.cloneNode(true));
  const gradient=template.content.querySelector('linearGradient')?.id;
  const paths = pause.map(() => {
    const path = document.createElementNS(ns, 'path');
    path.setAttribute('fill', 'none');
    path.setAttribute('stroke', 'var(--icon-stroke,currentColor)');
    path.setAttribute('stroke-width', '3');
    path.setAttribute('stroke-linejoin', 'round');
    path.setAttribute('stroke-linecap', 'round');
    svg.append(path);
    return path;
  });
  // Paint both fills above both outlines. Overlapping contours then form one
  // soft silhouette instead of drawing a hard internal seam through the morph.
  const fills = paths.map(() => {
    const path = document.createElementNS(ns, 'path');
    path.setAttribute('fill', gradient?`url(#${gradient})`:'var(--icon-fill,currentColor)');
    svg.append(path);
    return path;
  });
  host.replaceChildren(svg);
  host.dataset.morphing = 'true';
  const began = performance.now();
  let frame = 0;
  function tick(now: number) {
    const t = Math.min(1, (now - began) / 360);
    const ease = t * t * (3 - 2 * t);
    const progress = start + (end - start) * ease;
    host.dataset.transportMorphProgress = String(progress);
    paths.forEach((path, i) => {
      const points = play.map((p, j) => p.map((v, k) => v + (pause[i][j][k] - v) * progress) as Point);
      const d = rounded(points, 1.6 + 1.6 * progress);
      path.setAttribute('d', d);
      fills[i].setAttribute('d', d);
    });
    if (t < 1) frame = requestAnimationFrame(tick);
    else { host.innerHTML = finalMarkup; delete host.dataset.morphing; }
  }
  tick(began);
  return () => { cancelAnimationFrame(frame); delete host.dataset.morphing; };
}
