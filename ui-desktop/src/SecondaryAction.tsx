import {forwardRef,type ButtonHTMLAttributes} from 'react';
import {AssetIcon} from './AssetIcon';
type Props=ButtonHTMLAttributes<HTMLButtonElement>&{icon?:string;expanded?:boolean};
const SecondaryAction=forwardRef<HTMLButtonElement,Props>(function SecondaryAction({icon,expanded,children,className='',...props},ref){
 return <button {...props} ref={ref} className={`soft-action ${className}`}>
  {icon&&<AssetIcon name={icon}/>}{children}
  {expanded!==undefined&&<svg className="action-chevron" data-expanded={expanded} viewBox="0 0 16 16" aria-hidden="true"><path d="m4 6 4 4 4-4"/></svg>}
 </button>;
});
export default SecondaryAction;
