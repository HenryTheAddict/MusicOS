/** Never invent an owner while bootstrap is pending. */
export function personalName(name:string|undefined,ready=true){return !ready?'':name?.trim()||'Your library';}
