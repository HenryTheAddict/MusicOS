/** Smooth connections through the actual slider-handle centers (12px handles). */
export function eqConnections(gains:number[],height=68):string {
 const points=gains.map((gain,i)=>[(i+.5)*180/gains.length,6+(12-Math.max(-12,Math.min(12,gain)))/24*(height-12)]);
 if(!points.length)return '';
 let path=`M ${points[0][0]} ${points[0][1]}`;
 for(let i=1;i<points.length;i++){const a=points[i-1],b=points[i],mid=(a[0]+b[0])/2;path+=` C ${mid} ${a[1]}, ${mid} ${b[1]}, ${b[0]} ${b[1]}`;}
 return path;
}
