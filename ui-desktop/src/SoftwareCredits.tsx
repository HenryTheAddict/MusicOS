import {useEffect,useRef,useState} from 'react';
import credits from '../../credits.md?raw';
import catalog from '../../assets/visualizers/manifest.json';
import {Icon} from './components';
import './credits.css';
const notices=import.meta.glob<string>('../../third_party/licenses/**/*',{query:'?raw',import:'default'});
export default function Credits(){
 const [open,setOpen]=useState(false),[notice,setNotice]=useState(''),[licenseText,setLicenseText]=useState('');const dialog=useRef<HTMLDialogElement>(null),opener=useRef<HTMLButtonElement>(null);
 useEffect(()=>{if(!open)return;const node=dialog.current!;node.showModal();return()=>{node.close();opener.current?.focus({preventScroll:true});};},[open]);
 useEffect(()=>{let alive=true;setLicenseText('');if(notice)void notices[notice]().then(text=>{if(alive)setLicenseText(text);}).catch(()=>{if(alive)setLicenseText('This notice could not be opened. It is also bundled with the app’s licenses.');});return()=>{alive=false;};},[notice]);
 const entries=Object.keys(notices).sort();
 return <><button ref={opener} className="soft-action" onClick={()=>setOpen(true)}><Icon name="Song"/>Credits &amp; licenses</button>{open&&<dialog className="credits-dialog" ref={dialog} onCancel={e=>{e.preventDefault();setOpen(false);}}><header><h2>Credits &amp; licenses</h2><button className="icon-button" aria-label="Close credits" onClick={()=>setOpen(false)}><Icon name="controlClose"/></button></header><div className="credits-body"><p>Made with the work of these artists, designers, and open-source contributors.</p><h3>Visualizers</h3><p>Butterchurn by Jordan Berg. MilkDrop by Ryan Geiss. The original preset artists retain their attribution:</p><ul>{catalog.presets.map(p=><li key={p.id}><strong>{p.name}</strong><span>{p.original_filename.replace(/\.json$/,'')}</span><small>{p.authors}</small></li>)}</ul><details><summary>Complete credits</summary><pre>{credits}</pre></details><h3>License texts</h3><label>Choose a notice<select value={notice} onChange={e=>setNotice(e.target.value)}><option value="">Select a license</option>{entries.map(path=><option key={path} value={path}>{path.replace('../../third_party/licenses/','')}</option>)}</select></label>{notice&&<pre tabIndex={0}>{licenseText||'Opening license…'}</pre>}</div></dialog>}</>;
}
