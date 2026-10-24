/** Normalize computed colors from both WebKit and Chromium to opaque sRGB hex. */
export function cssColorHex(value:string,fallback:string):string {
 const color=value.trim();if(/^#[0-9a-f]{6}$/i.test(color))return color;
 const srgb=/^color\(srgb\s+([^/)]+)(?:\s*\/[^)]+)?\)$/i.exec(color);
 const rgb=/^rgba?\(([^/)]+)(?:\s*\/[^)]+)?\)$/i.exec(color);
 const parts=(srgb?.[1]??rgb?.[1])?.trim().split(/[\s,]+/).slice(0,3);
 if(parts?.length===3){const numbers=parts.map(part=>part.endsWith('%')?parseFloat(part)/100*255:parseFloat(part)*(srgb?255:1));if(numbers.every(Number.isFinite))return '#'+numbers.map(n=>Math.round(Math.max(0,Math.min(255,n))).toString(16).padStart(2,'0')).join('');}
 // Computed color-mix may use another supported CSS color space. Canvas converts
 // those colors into the same sRGB representation used by the contrast helpers.
 const canvas=document.createElement('canvas');canvas.width=canvas.height=1;const context=canvas.getContext('2d');if(!context)return fallback;
 context.fillStyle=fallback;context.fillStyle=color;context.fillRect(0,0,1,1);const rgba=context.getImageData(0,0,1,1).data;return rgba[3]?'#'+[...rgba].slice(0,3).map(n=>n.toString(16).padStart(2,'0')).join(''):fallback;
}
