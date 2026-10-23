import {Component,createRef,type ReactNode} from 'react';
/** Capture outgoing pixels before React updates; position ticks never create snapshots. */
export default class TrackCrossfade extends Component<{identity?:number;children:ReactNode}> {
 private root=createRef<HTMLDivElement>();
 private ghost:HTMLElement|null=null;
 private animation:Animation|null=null;
 getSnapshotBeforeUpdate(previous:Readonly<{identity?:number;children:ReactNode}>){
  const root=this.root.current;
  if(previous.identity===this.props.identity||!root||root.closest('[inert]')||matchMedia('(prefers-reduced-motion: reduce)').matches)return null;
  return root.firstElementChild?.cloneNode(true) as HTMLElement|null;
 }
 componentDidUpdate(_previous:Readonly<{identity?:number;children:ReactNode}>,_state:unknown,snapshot:HTMLElement|null){
  if(!snapshot)return;
  this.animation?.cancel();this.ghost?.remove();this.ghost=snapshot;
  snapshot.className='mini-track-ghost';snapshot.inert=true;snapshot.setAttribute('aria-hidden','true');snapshot.querySelectorAll('[id]').forEach(node=>node.removeAttribute('id'));
  this.root.current?.append(snapshot);
  const animation=snapshot.animate([{opacity:1},{opacity:0}],{duration:180,easing:'ease-out'});this.animation=animation;
  void animation.finished.then(()=>{snapshot.remove();if(this.ghost===snapshot)this.ghost=null;}).catch(()=>{});
 }
 componentWillUnmount(){this.animation?.cancel();this.ghost?.remove();}
 render(){return <div ref={this.root} className="mini-track-fade"><div className="mini-track-content">{this.props.children}</div></div>;}
}
