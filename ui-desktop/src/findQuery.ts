/** Local, ordered-list matching. No provider requests or playback mutations. */
export type FindRecord={title?:string;artist?:string;album?:string;genre?:string;year?:number;favorited?:boolean;available?:boolean;text?:string};
const fold=(value:string)=>value.normalize('NFKD').replace(/\p{M}/gu,'').toLocaleLowerCase().replace(/[’']/g,'').replace(/[^\p{L}\p{N}]+/gu,' ').trim().replace(/\s+/g,' ');
function near(a:string,b:string){
 if(Math.abs(a.length-b.length)>1)return false;
 let i=0,j=0,edits=0;
 while(i<a.length&&j<b.length){if(a[i]===b[j]){i++;j++;continue;}if(++edits>1)return false;if(a[i]===b[j+1]&&a[i+1]===b[j]){i+=2;j+=2;continue;}if(a.length>=b.length)i++;if(b.length>=a.length)j++;}
 return edits+(i<a.length||j<b.length?1:0)<=1;
}
export function compileListFind(query:string){
 const tokens=[...query.matchAll(/(-?)(?:(title|artist|album|genre|year|is):)?(?:"([^"]*)(?:"|$)|(\S+))/gi)].map(match=>({negative:!!match[1],field:match[2]?.toLowerCase(),raw:match[3]??match[4],phrase:match[3]!==undefined}));
 if(!tokens.length)return (_record:FindRecord)=>true;
 return (record:FindRecord)=>{
  const text=fold([record.title,record.artist,record.album,record.genre,record.year,record.text].filter(value=>value!==undefined).join(' '));
  return tokens.every(token=>{
   let matched=false;
   if(token.field==='is')matched=['liked','favorite','favorited'].includes(token.raw.toLowerCase())?!!record.favorited:token.raw.toLowerCase()==='available'?record.available===true:token.raw.toLowerCase()==='unavailable'?record.available===false:false;
   else if(token.field==='year'){
    const range=token.raw.match(/^(\d{4})(?:-(\d{4}))?$/);matched=!!range&&record.year!==undefined&&record.year>=Number(range[1])&&record.year<=Number(range[2]??range[1]);
   }else{
    const needle=fold(token.raw),haystack=token.field?fold(String(record[token.field as 'title'|'artist'|'album'|'genre']??'')):text;
    matched=!needle||haystack.includes(needle)||(!token.phrase&&needle.length>=4&&haystack.split(' ').some(word=>near(needle,word)));
   }
   return token.negative?!matched:matched;
  });
 };
}
