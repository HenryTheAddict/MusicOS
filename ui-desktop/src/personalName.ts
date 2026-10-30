import {product} from './product';
/** Never invent an owner while bootstrap is pending. */
export function personalName(name:string|undefined,ready=true){return !ready?'':name?.trim()||product.defaults.libraryLabel;}
