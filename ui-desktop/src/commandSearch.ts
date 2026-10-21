/** Prefer a named release/artist/title over matches inherited from its credits. */
export function searchScore(query:string,title:string){
 const value=title.trim().toLocaleLowerCase();
 return value===query?0:value.startsWith(query)?1:value.includes(query)?2:3;
}
export function groupMusicResults<T extends {group:string;score:number;title:string}>(items:T[]):T[]{
 const order=['Albums','Artists','Songs','Collections'];
 const groups=order.map(group=>({group,items:items.filter(item=>item.group===group).sort((a,b)=>a.score-b.score||a.title.localeCompare(b.title)).slice(0,group==='Songs'?12:8)})).filter(group=>group.items.length);
 groups.sort((a,b)=>a.items[0].score-b.items[0].score||order.indexOf(a.group)-order.indexOf(b.group));
 return groups.flatMap(group=>group.items);
}
