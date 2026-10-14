// Local, editable labels. Names do not change ranking or imply a model diagnosis.
export function seedVibeTitle(title?:string){return title?Array.from(title.replace(/\s*[([](?:feat\.?|ft\.?|with|explicit)\b[^)\]]*[)\]]/gi,'').trim().replace(/[.!?]+$/,'')).slice(0,48).join(''):'';}
export function sparkNames(seedTitle?:string,mood?:string|null,hue?:number){
 const title=seedVibeTitle(seedTitle);return title?[`${title}-esque`]:[];
}
export const moodHue:Record<string,number>={Chilled:195,Energetic:15,Melancholic:245,Bright:55,Focused:155,Dreamy:285};
export function hexHue(hex:string){const values=hex.replace('#','').match(/.{2}/g)?.map(v=>parseInt(v,16)/255);if(!values||values.length!==3||values.some(v=>!Number.isFinite(v)))return null;const [r,g,b]=values,max=Math.max(...values),min=Math.min(...values),d=max-min;if(d===0)return 210;const h=max===r?(g-b)/d+(g<b?6:0):max===g?(b-r)/d+2:(r-g)/d+4;return h*60;}
