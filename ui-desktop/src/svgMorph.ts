// Production adaptation of svg-morph-engine.html: equidistant contour samples,
// cyclic alignment, and collapse/fade for appearing or disappearing contours.
// Preparation is cached; only an active, finite transition requests frames.
type Point = { x: number; y: number };
type Contour = { points: Point[]; closed: boolean; fill: string; stroke: string; width: number; rule: string };
const ns = "http://www.w3.org/2000/svg";
const cache = new Map<string, Contour[]>();

function sample(markup: string, color:string, fill:string, stroke:string): Contour[] {
  const host = document.createElement("div");
  host.style.cssText = "position:fixed;left:-10000px;top:0;visibility:hidden;pointer-events:none";
  host.style.color=color;host.style.setProperty("--icon-fill",fill);host.style.setProperty("--icon-stroke",stroke);
  host.innerHTML = markup;
  document.body.append(host);
  try {
    const svg = host.querySelector("svg")!;
    svg.setAttribute("width", "32");
    svg.setAttribute("height", "32");
    const contours: Contour[] = [];
    svg.querySelectorAll<SVGGeometryElement>("path,rect,circle,ellipse,polygon,polyline,line").forEach(element => {
      if (element.closest("defs,clipPath,mask")) return;
      const style = getComputedStyle(element);
      if (style.fill === "none" && style.stroke === "none") return;
      // Compound paths must stay separate: no artificial bridges across holes.
      const pieces = element.tagName === "path"
        ? element.getAttribute("d")?.match(/[Mm][^Mm]*/g) ?? []
        : [null];
      for (const piece of pieces) {
        const geometry = piece ? document.createElementNS(ns, "path") : element;
        if (piece) geometry.setAttribute("d", piece);
        const length = geometry.getTotalLength();
        if (!Number.isFinite(length) || length <= 0) continue;
        const matrix = svg.getCTM()!.inverse().multiply(element.getCTM()!);
        const closed = piece ? /[Zz]\s*$/.test(piece) : !["line", "polyline"].includes(element.tagName);
        const points = Array.from({ length: 64 }, (_, i) => {
          const point = geometry.getPointAtLength(length * i / (closed ? 64 : 63));
          const transformed = new DOMPoint(point.x, point.y).matrixTransform(matrix);
          return { x: transformed.x * 32 / svg.viewBox.baseVal.width, y: transformed.y * 32 / svg.viewBox.baseVal.height };
        });
        contours.push({ points, closed, fill: style.fill, stroke: style.stroke, width: parseFloat(style.strokeWidth)*32/svg.viewBox.baseVal.width, rule: style.fillRule });
      }
    });
    return contours;
  } finally { host.remove(); }
}

function prepared(markup:string, host:HTMLElement) {
  const style=getComputedStyle(host), fill=style.getPropertyValue('--icon-fill').trim()||style.color, stroke=style.getPropertyValue('--icon-stroke').trim()||style.color;
  const key=JSON.stringify([markup,style.color,fill,stroke]);
  if(!cache.has(key)){if(cache.size>=128)cache.delete(cache.keys().next().value!);cache.set(key,sample(markup,style.color,fill,stroke));}
  return cache.get(key)!;
}
function center(points: Point[]) {
  return points.reduce((p, q) => ({ x: p.x + q.x / points.length, y: p.y + q.y / points.length }), { x: 0, y: 0 });
}
function aligned(a: Point[], b: Point[], closed: boolean) {
  if (!closed) return b;
  let best = b, distance = Infinity;
  // Also consider reversed winding so a source export cannot twist the shape.
  for (const direction of [b, [...b].reverse()]) {
    for (let offset = 0; offset < b.length; offset++) {
      const score = a.reduce((sum, p, i) => {
        const q = direction[(i + offset) % b.length];
        return sum + (p.x - q.x) ** 2 + (p.y - q.y) ** 2;
      }, 0);
      if (score < distance) { distance = score; best = direction.slice(offset).concat(direction.slice(0, offset)); }
    }
  }
  return best;
}
function path(points: Point[], closed: boolean) {
  // Dense samples retain the source's rounded corners without spline overshoot.
  return points.map((p, i) => `${i ? "L" : "M"}${p.x.toFixed(3)},${p.y.toFixed(3)}`).join(" ") + (closed ? " Z" : "");
}
function color(a: string, b: string, t: number) {
  const x = a.match(/[\d.]+/g)?.map(Number), y = b.match(/[\d.]+/g)?.map(Number);
  if (!x || !y) return t < .5 ? a : b;
  return `rgb(${x.slice(0, 3).map((v, i) => Math.round(v + (y[i] - v) * t)).join(",")})`;
}

export function morphSvg(host: HTMLElement, from: string, to: string, duration = 180) {
  const a = prepared(from,host), b = prepared(to,host);
  if (!a.length || !b.length) { host.innerHTML = to; return () => {}; }
  const svg = document.createElementNS(ns, "svg");
  svg.setAttribute("viewBox", "0 0 32 32");
  svg.setAttribute("aria-hidden", "true");
  const pairs = Array.from({ length: Math.max(a.length, b.length) }, (_, i) => {
    const left = a[i], right = b[i];
    const start = left?.points ?? right.points.map(() => center(a[0].points));
    const end = right?.points ?? left.points.map(() => center(b[0].points));
    const target = aligned(start, end, !!left?.closed && !!right?.closed);
    const element = document.createElementNS(ns, "path");
    element.setAttribute("stroke-linecap", "round");
    element.setAttribute("stroke-linejoin", "round");
    element.setAttribute("fill-rule", (right ?? left).rule);
    svg.append(element);
    return { start, target, left, right, element };
  });
  host.replaceChildren(svg);
  host.dataset.morphing = "true";
  const started = performance.now();
  let frame = 0;
  const render = (now: number) => {
    const progress = Math.min(1, (now - started) / duration);
    const t = progress < .5 ? 4 * progress ** 3 : 1 - (-2 * progress + 2) ** 3 / 2;
    for (const { start, target, left, right, element } of pairs) {
      element.setAttribute("d", path(start.map((p, i) => ({ x: p.x + (target[i].x - p.x) * t, y: p.y + (target[i].y - p.y) * t })), (right ?? left).closed));
      element.setAttribute("fill", color((left ?? right).fill, (right ?? left).fill, t));
      element.setAttribute("stroke", color((left ?? right).stroke, (right ?? left).stroke, t));
      element.setAttribute("stroke-width", String((left ?? right).width + ((right ?? left).width - (left ?? right).width) * t));
      element.setAttribute("opacity", String(!left ? t : !right ? 1 - t : 1));
    }
    if (progress < 1) frame = requestAnimationFrame(render);
    else { host.innerHTML = to; delete host.dataset.morphing; }
  };
  render(started);
  return () => { cancelAnimationFrame(frame); delete host.dataset.morphing; };
}
