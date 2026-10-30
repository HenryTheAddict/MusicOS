import configuration from '../../product.config.json';
import type {Appearance,Preferences} from './types';
/** Shared build-time product settings. User choices always take precedence. */
export const product={...configuration,defaults:{...configuration.defaults,appearance:configuration.defaults.appearance as Appearance,interfacePercent:configuration.defaults.interfacePercent as Preferences['interface_percent'],miniPlayerDensity:configuration.defaults.miniPlayerDensity as NonNullable<Preferences['mini_player_density']>}};
